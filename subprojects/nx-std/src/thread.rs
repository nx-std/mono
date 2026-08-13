//! Threads, in the shape `std` gives them.
//!
//! This module is the workspace's stand-in for `std::thread`. It exists because
//! the layer underneath it, [`nx_sys_thread`], is a platform abstraction rather
//! than a caller-facing API: its thread body is a type-erased
//! [`ThreadBody`] returning nothing, its [`nx_sys_thread::thread::spawn`] is
//! `unsafe` because a caller may supply the stack, and its
//! [`nx_sys_thread::thread::Thread`] join yields no value. All three are right
//! for the layer that owns the kernel resource and wrong for a caller, who
//! wants to hand over a closure and get its return value back.
//!
//! # The split, and why it is the one `std` uses
//!
//! `std` draws the same line. `std::sys::thread` creates a thread from a boxed,
//! type-erased body and joins it without a value; every generic piece (the
//! builder, `spawn`, and a `JoinHandle<T>` that yields `T`) lives in
//! `std::thread`, above it. The generic parameter never reaches the platform
//! layer, so that layer needs one trampoline rather than one per closure type,
//! and the code that talks to the kernel stays monomorphic.
//!
//! The mechanism that carries a value across the boundary is the same one `std`
//! uses: the caller's closure is wrapped in a body that captures a shared result
//! slot, writes into it, and drops its share on the way out. The platform layer
//! sees only a `FnOnce()`.
//!
//! # What a joined thread can report
//!
//! Every crate here builds with `panic = "abort"`, so a spawned thread cannot
//! unwind: there is no panic payload for a joiner to receive, and
//! [`JoinHandle::join`] fails only when the platform layer could not join the
//! thread. That is why its error is a single wrapped
//! [`nx_sys_thread::thread::JoinError`] rather than `std`'s
//! `Box<dyn Any + Send>`.

use alloc::{
    boxed::Box,
    sync::Arc,
};
use core::{
    cell::UnsafeCell,
    num::NonZero,
    time::Duration,
};

use nx_sys_thread::thread::{
    self as sys,
    StackSpec,
    ThreadBody,
};
pub use nx_sys_thread::thread::{
    CoreId,
    DEFAULT_STACK_SIZE,
    Priority,
    ThreadId,
};

/// Spawns a new thread running `f` and returns a handle to it.
///
/// The thread is created with every scheduling parameter at its default; use
/// [`Builder`] to override the stack size, priority, or CPU core. Dropping the
/// returned handle without joining detaches the thread, which then reclaims
/// itself once it exits.
///
/// # This returns a `Result`, where `std` panics
///
/// `std::thread::spawn` is documented to panic when the thread cannot be
/// created, on the reasoning that a caller who wants to handle it can reach for
/// `Builder::spawn`. That reasoning does not survive the move to this target.
/// Every crate here builds with `panic = "abort"`, so the panic would end the
/// process outright: nothing unwinds, no `Drop` runs, and open kernel handles
/// and mapped pages stay live until the kernel tears the process down. Thread
/// creation fails on conditions a program can actually meet: an exhausted
/// address space, no memory for the stack. Aborting on one would turn a
/// recoverable condition into a crash, which this workspace does not do.
///
/// The signature is therefore the same as [`Builder::spawn`]'s; this function is
/// the shorthand for the default configuration, not a panicking variant.
///
/// # Errors
///
/// Returns [`SpawnError`] if the platform layer could not bring the thread up.
/// Nothing is left behind on failure: the closure and its captures are dropped
/// before returning.
pub fn spawn<F, T>(f: F) -> Result<JoinHandle<T>, SpawnError>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    Builder::new().spawn(f)
}

/// Thread factory, for configuring a thread before spawning it.
///
/// Mirrors `std::thread::Builder`: every parameter has a default, so a caller
/// overrides only what it cares about. The Horizon scheduling parameters `std`
/// has no counterpart for, [`Priority`] and [`CoreId`], are set here too.
///
/// Defaults: a [`DEFAULT_STACK_SIZE`] stack, [`Priority::DEFAULT`], and
/// [`CoreId::PROCESS_DEFAULT`].
#[derive(Debug, Clone, Copy)]
pub struct Builder {
    /// Requested usable stack size, in bytes.
    stack_size: usize,
    /// Horizon scheduling priority.
    priority: Priority,
    /// Target CPU core.
    core_id: CoreId,
}

impl Builder {
    /// Creates a builder with every parameter at its default (see the
    /// [type docs](Builder)).
    pub const fn new() -> Self {
        Self {
            stack_size: DEFAULT_STACK_SIZE,
            priority: Priority::DEFAULT,
            core_id: CoreId::PROCESS_DEFAULT,
        }
    }

    /// Sets the size of the stack, in bytes, for the spawned thread.
    ///
    /// The value is the usable stack, before the per-thread control regions the
    /// platform layer reserves on top of it.
    pub const fn stack_size(mut self, size: usize) -> Self {
        self.stack_size = size;
        self
    }

    /// Sets the Horizon scheduling priority for the spawned thread.
    pub const fn priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    /// Sets the CPU core the spawned thread runs on.
    pub const fn core_id(mut self, core_id: CoreId) -> Self {
        self.core_id = core_id;
        self
    }

    /// Spawns a thread running `f`, returning a handle that yields its value.
    ///
    /// `F` is bounded `FnOnce() -> T + Send + 'static` and `T` is
    /// `Send + 'static`: `FnOnce` because a thread body runs exactly once,
    /// `Send` because the closure and its return value cross to another thread,
    /// and `'static` because the closure must not borrow the caller's frame.
    ///
    /// # Errors
    ///
    /// Returns [`SpawnError`] if the platform layer could not bring the thread
    /// up: an exhausted address space, no memory for the stack, or a kernel
    /// refusal. Nothing is left behind on failure: the closure and its captures
    /// are dropped before returning.
    pub fn spawn<F, T>(self, f: F) -> Result<JoinHandle<T>, SpawnError>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let packet = Arc::new(Packet::new());
        let thread_packet = Arc::clone(&packet);

        // Wrapping the caller's closure in a body that captures its own result
        // slot is what keeps the platform layer non-generic: what crosses down
        // is a `FnOnce()`, with `T` sealed inside the capture.
        let body: ThreadBody = Box::new(move || {
            let value = f();
            // SAFETY: this body runs exactly once, on the spawned thread, and
            // is the only writer of the packet; the joiner reads it only after
            // observing the thread's exit, so the write is ordered before it.
            unsafe { thread_packet.fill(value) };
        });

        let config = sys::Builder::new()
            .stack(StackSpec::Auto(self.stack_size))
            .priority(self.priority)
            .core_id(self.core_id)
            .build_spawn();

        // SAFETY: the stack is `StackSpec::Auto`, so the platform layer
        // allocates and owns it; the caller-supplied-buffer clause of `spawn`'s
        // contract, the only obligation it states, does not apply.
        let thread = unsafe { sys::spawn(config, body) }.map_err(SpawnError)?;

        Ok(JoinHandle { thread, packet })
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

/// Error returned when [`Builder::spawn`] cannot bring the thread up.
#[derive(Debug, thiserror::Error)]
#[error("failed to spawn the thread")]
pub struct SpawnError(#[source] pub sys::SpawnError);

/// An owned handle to a spawned thread, yielding the value its closure produced.
///
/// Mirrors `std::thread::JoinHandle`. It is neither `Copy` nor `Clone`:
/// [`join`](Self::join) consumes the handle, so a thread is joined at most once
/// and a double join is a compile error. Dropping the handle without joining
/// *detaches* the thread, which reclaims itself once it exits, so an unjoined
/// handle leaks nothing.
pub struct JoinHandle<T: Send + 'static> {
    /// The platform-layer thread this handle owns.
    ///
    /// Its `Drop` is what detaches an unjoined thread, so this type needs no
    /// `Drop` of its own.
    thread: sys::Thread,
    /// The result slot the spawned body writes and [`join`](Self::join) reads.
    packet: Arc<Packet<T>>,
}

impl<T: Send + 'static> JoinHandle<T> {
    /// Waits for the thread to finish and returns the value its closure
    /// produced.
    ///
    /// # Errors
    ///
    /// Returns [`JoinError`] if the platform layer could not join the thread:
    /// either waiting for it to terminate failed, in which case it may still be
    /// running, or reclaiming its stack mapping and kernel handle did. The
    /// closure's value is not recovered in either case.
    pub fn join(self) -> Result<T, JoinError> {
        let Self { thread, packet } = self;

        thread.join().map_err(JoinError)?;

        // SAFETY: `join` returned `Ok`, so the thread ran its body to
        // completion, filling the packet, and dropped its `Arc` share before
        // exiting, which leaves this the sole owner of a filled packet.
        let value = Arc::into_inner(packet)
            .and_then(Packet::take)
            .expect("JoinHandle::join: an exited thread has filled and released its packet");

        Ok(value)
    }

    /// Returns `true` once the spawned thread has finished.
    ///
    /// The non-blocking counterpart to [`join`](Self::join): a `true` answer
    /// means `join` will not block. It reclaims nothing, so `join` is still what
    /// recovers the value.
    pub fn is_finished(&self) -> bool {
        self.thread.is_finished()
    }
}

/// Error returned when [`JoinHandle::join`] cannot join the thread.
#[derive(Debug, thiserror::Error)]
#[error("failed to join the thread")]
pub struct JoinError(#[source] pub sys::JoinError);

/// A handle to a thread, carrying its identity.
///
/// Mirrors `std::thread::Thread`, minus the name and the park state `std`
/// attaches to it. It is obtained from [`current`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Thread {
    /// The thread's process-unique, never-recycled identity.
    id: ThreadId,
}

impl Thread {
    /// Returns the thread's process-unique identifier.
    pub const fn id(&self) -> ThreadId {
        self.id
    }
}

/// Returns a handle to the calling thread, if it has one.
///
/// Unlike `std::thread::current`, this can answer `None`: identity comes from
/// the core state the platform layer installs, and a thread it neither created
/// nor adopted has none. Every thread this module spawns has one, as does the
/// main thread once the runtime has adopted it.
pub fn current() -> Option<Thread> {
    let ptr = sys::current()?;
    // SAFETY: `current` returns a pointer to the calling thread's own core
    // state, which stays live for as long as that thread runs, so it is valid
    // here. The only fields the running thread mutates concurrently (`state`,
    // `prev`, `next`) are atomics, so a shared reference over them is not a data
    // race, and `id` is fixed at creation and never written again.
    let id = unsafe { ptr.as_ref() }.id();
    Some(Thread { id })
}

/// Suspends the calling thread for *at least* `dur`.
pub fn sleep(dur: Duration) {
    sys::sleep(dur);
}

/// Yields the calling thread's remaining time slice to the scheduler.
///
/// The kernel reschedules another ready thread and may migrate the caller to a
/// different CPU core.
pub fn yield_now() {
    sys::yield_thread();
}

/// Returns the number of CPU cores this process may run threads on.
///
/// Reads the process's core mask, so it reports the cores the kernel actually
/// permits rather than the four the console has.
///
/// # Errors
///
/// Returns [`AvailableParallelismError::Query`] if the core mask could not be
/// read, and [`AvailableParallelismError::NoCores`] if it came back empty,
/// which would mean a process that may run on no core at all.
pub fn available_parallelism() -> Result<NonZero<usize>, AvailableParallelismError> {
    let mask = nx_svc::misc::get_info(
        nx_svc::misc::InfoType::CoreMask,
        nx_svc::raw::CUR_PROCESS_HANDLE,
    )
    .map_err(AvailableParallelismError::Query)?;

    // Each set bit is one permitted core, so the population count is the answer.
    // `count_ones` on a `u64` yields at most 64, which fits every `usize` this
    // target has, so the saturating fallback is unreachable; it stands in for an
    // `as` cast rather than handling a real conversion failure.
    let count = usize::try_from(mask.count_ones()).unwrap_or(usize::MAX);
    NonZero::new(count).ok_or(AvailableParallelismError::NoCores)
}

/// Errors returned by [`available_parallelism`].
#[derive(Debug, thiserror::Error)]
pub enum AvailableParallelismError {
    /// The process core mask could not be read from the kernel.
    #[error("failed to query the process core mask")]
    Query(#[source] nx_svc::misc::GetInfoError),
    /// The core mask came back empty.
    ///
    /// The kernel always grants a runnable process at least one core, so this
    /// reports a mask that cannot be acted on rather than a real configuration.
    #[error("the process core mask permits no cores")]
    NoCores,
}

/// The result slot a spawned thread writes and its joiner reads.
///
/// Shared through an [`Arc`] between the [`JoinHandle`] and the thread body.
/// This is the piece that carries `T` across a platform boundary that does not
/// know about `T`: the body captures its share, so the value travels inside the
/// closure rather than through the layer below.
struct Packet<T> {
    /// The closure's return value; `None` until the body fills it.
    ///
    /// An [`UnsafeCell`] because the writing thread holds only a shared
    /// reference through the `Arc`. The write happens-before the joiner's read,
    /// which is ordered after the kernel's termination signal, so the access is
    /// not a data race.
    result: UnsafeCell<Option<T>>,
}

impl<T> Packet<T> {
    /// Creates an empty slot.
    const fn new() -> Self {
        Self {
            result: UnsafeCell::new(None),
        }
    }

    /// Stores the value the closure produced.
    ///
    /// # Safety
    ///
    /// Must be called at most once, on the spawned thread, before that thread
    /// exits: it is the write the joiner's read is ordered against, and a second
    /// call would race a joiner that has already observed the exit.
    unsafe fn fill(&self, value: T) {
        // SAFETY: by the contract this is the only write, and it precedes the
        // exit the joiner's read is ordered after, so no reference aliases the
        // slot for the duration of this store.
        unsafe { *self.result.get() = Some(value) };
    }

    /// Takes the stored value, consuming the slot.
    fn take(self) -> Option<T> {
        self.result.into_inner()
    }
}

// SAFETY: the packet crosses to the spawned thread and its value crosses back,
// which is exactly what `T: Send` licenses. The `UnsafeCell` is written once by
// the spawned thread and read once by the joiner, ordered by the kernel's
// termination signal, so sharing it is not a data race.
unsafe impl<T: Send> Send for Packet<T> {}
// SAFETY: see the `Send` impl above; the same ordering makes a shared
// `&Packet` sound to hold on more than one thread.
unsafe impl<T: Send> Sync for Packet<T> {}
