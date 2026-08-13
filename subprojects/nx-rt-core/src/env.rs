//! # Runtime Environment State
//!
//! Kind-agnostic container for the runtime environment every Switch
//! executable shares: the main-thread and own-process handles, the heap
//! override region, `argv`, syscall-availability hints, the applet type,
//! loader-supplied service overrides, the random seed, and the chain-loading
//! buffers.
//!
//! Each output kind parses its own startup source — the homebrew loader
//! configuration for an NRO, a build-time profile for an NSO — and fills this
//! container exactly once through [`init_once`]. Every other consumer reads it
//! back through the accessor functions below, which are sound precisely
//! because the state is written once and immutable thereafter.

use core::{
    cell::UnsafeCell,
    ffi::{
        CStr,
        c_char,
        c_void,
    },
    ptr::{
        self,
        NonNull,
    },
};

pub use nx_sf::ServiceName;
use nx_svc::{
    ipc::Handle as ServiceHandle,
    process::Handle as ProcessHandle,
    thread::Handle as ThreadHandle,
};
use nx_sys_sync::{
    Mutex,
    Once,
};

pub mod hos_version;
pub mod main_thread;
mod syscall_hint;

pub use self::syscall_hint::SyscallHints;
#[cfg(feature = "ffi")]
use crate::error::{
    LibnxError,
    libnx_error,
};

/// Loader return function type
pub type LoaderReturnFn = Option<unsafe extern "C" fn(i32) -> !>;

/// Maximum number of service overrides the loader may supply
pub const MAX_SERVICE_OVERRIDES: usize = 32;

/// Global environment state (immutable after initialization)
static ENV_STATE: EnvStateWrapper = EnvStateWrapper::new();

/// Initialization guard to ensure the env state is populated exactly once
static ENV_INIT: Once = Once::new();

/// Where to return to when the process is done (mutable at runtime)
static EXIT_FUNC: ExitFunc = ExitFunc::new();

/// Serializes writes into the loader's chain-load buffers.
static NEXT_LOAD_LOCK: Mutex = Mutex::new();

/// Populate the global environment state exactly once.
///
/// `main_thread` is the handle the kernel gave the process for the thread it
/// started on. It is a parameter rather than something the `populate` closure
/// fills because it does not come from the startup source: an entry crate that
/// finds nothing to parse still knows it, and the steps that run straight
/// after startup read it back before anything else.
///
/// The `populate` closure receives exclusive `&mut` access to the [`EnvState`]
/// container and is responsible for filling the rest from whatever startup
/// source the calling entry crate owns: the homebrew loader configuration for
/// an NRO, a build-time profile for an NSO, and so on.
///
/// Subsequent calls are no-ops: the state is written once here and is
/// read-only afterwards, which is what makes the unsynchronized accessor
/// functions below sound.
pub fn init_once(main_thread: ThreadHandle, populate: impl FnOnce(&mut EnvState)) {
    ENV_INIT.call_once(|| {
        // SAFETY: `Once::call_once` guarantees this runs exactly once with
        // exclusive access; no accessor can observe the state mid-write.
        let state = unsafe { &mut *ENV_STATE.get() };
        state.main_thread_handle = Some(main_thread);
        populate(state);
    });
}

/// Get loader info string pointer and size
pub fn loader_info() -> Option<(NonNull<c_char>, u64)> {
    // SAFETY: ENV_STATE is initialized once via init_once() before any other
    // function is called. After initialization, the state is read-only.
    let state = unsafe { ENV_STATE.get_ref() };
    state.loader_info
}

/// The handle of the thread the process started on.
///
/// # Panics
///
/// Panics when the runtime environment has not been set up yet. [`init_once`]
/// takes the handle as an argument, so every startup path records it whether
/// or not its startup source described one; reaching this beforehand means the
/// startup sequence itself is out of order.
pub fn main_thread_handle() -> ThreadHandle {
    // SAFETY: ENV_STATE is initialized once via init_once() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    state
        .main_thread_handle
        .expect("the runtime environment has not been initialized")
}

/// Returns true if running as NSO, false if NRO
pub fn is_nso() -> bool {
    // SAFETY: ENV_STATE is initialized once via init_once() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    state.is_nso
}

/// Get heap override address and size if present
///
/// Returns `Some((addr, size))` if the homebrew loader provided a heap override,
/// or `None` if running without a heap override.
pub fn heap_override() -> Option<(NonNull<c_void>, usize)> {
    // SAFETY: ENV_STATE is initialized once via init_once() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    state.heap_override
}

/// The argument string the startup source supplied, if it supplied one.
///
/// Handed out as the mutable buffer the loader gave this process, because that
/// is what it is: the entry crate that installs the command line terminates
/// each argument in place rather than copying the string somewhere writable.
pub fn argv() -> Option<NonNull<c_char>> {
    // SAFETY: ENV_STATE is initialized once via init_once() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    state.argv
}

/// Which syscalls the startup source said this process may issue.
///
/// A startup source that says nothing leaves every syscall unhinted, which is
/// the same answer as a source that named none: the hints are a permission
/// list, so the absence of an entry is the absence of permission rather than a
/// missing fact.
pub fn syscall_hints() -> SyscallHints {
    // SAFETY: ENV_STATE is initialized once via init_once() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    state.syscall_hints
}

/// Get process handle if present
pub fn own_process_handle() -> Option<ProcessHandle> {
    // SAFETY: ENV_STATE is initialized once via init_once() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    state.process_handle
}

/// Set exit function pointer
pub fn set_exit_func_ptr(func: LoaderReturnFn) {
    EXIT_FUNC.set(func);
}

/// Get exit function pointer
pub fn exit_func_ptr() -> LoaderReturnFn {
    EXIT_FUNC.get()
}

/// Get last load result
pub fn last_load_result() -> u32 {
    // SAFETY: ENV_STATE is initialized once via init_once() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    state.last_load_result
}

/// Get random seed if present
pub fn random_seed() -> Option<[u64; 2]> {
    // SAFETY: ENV_STATE is initialized once via init_once() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    state.random_seed
}

/// Get user ID storage pointer if present
pub fn user_id_storage() -> Option<NonNull<AccountUid>> {
    // SAFETY: ENV_STATE is initialized once via init_once() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    state.user_id_storage
}

/// Returns true if chain loading is supported
pub fn has_next_load() -> bool {
    // SAFETY: ENV_STATE is initialized once via init_once() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    state.next_load.is_some()
}

/// Get service overrides as a slice of Options (first `count` are Some)
pub fn service_overrides() -> &'static [Option<ServiceOverride>] {
    // SAFETY: ENV_STATE is initialized once via init_once() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    &state.service_overrides[..state.service_override_count]
}

/// Get applet type
pub fn applet_type() -> AppletType {
    // SAFETY: ENV_STATE is initialized once via init_once() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    state.applet_type
}

/// Returns true if APT workaround is active (APT is broken and should not be used)
pub fn applet_workaround() -> bool {
    // SAFETY: ENV_STATE is initialized once via init_once() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };
    state.applet_workaround
}

/// Names the program to run once this one exits.
///
/// The request is written into the loader's own buffers, which is the only
/// place it can be read from: by the time the loader looks, this program is
/// gone. A copy kept here instead would be discarded with it, and the loader
/// would fall back to whatever it runs when nothing was asked for.
///
/// A program started without a command line still gets one; it is empty.
///
/// # Errors
///
/// Returns an error when the loader runs nothing after this program, or when
/// the request does not fit the buffers it provided.
pub fn set_next_load(path: &CStr, argv: Option<&CStr>) -> Result<(), SetNextLoadError> {
    // SAFETY: ENV_STATE is initialized once via init_once() and is read-only after that.
    let state = unsafe { ENV_STATE.get_ref() };

    let Some(next_load) = state.next_load else {
        return Err(SetNextLoadError::Unsupported);
    };

    // Two callers writing at once would interleave their bytes into one
    // request, and the loader would run neither program.
    NEXT_LOAD_LOCK.lock();
    let written = next_load.write(path, argv.unwrap_or(c""));
    NEXT_LOAD_LOCK.unlock();

    written
}

/// Error returned by [`set_next_load`].
#[derive(Debug, thiserror::Error)]
pub enum SetNextLoadError {
    /// The loader offered no way to name the program to run next.
    #[error("the loader does not run another program after this one")]
    Unsupported,
    /// The path or the command line is longer than the loader's buffer for it.
    #[error("the request does not fit the loader's buffers")]
    TooLong,
}

#[cfg(feature = "ffi")]
impl crate::error::ToResultCode for SetNextLoadError {
    fn to_rc(self) -> crate::error::ResultCode {
        match self {
            Self::Unsupported => libnx_error(LibnxError::NotInitialized),
            Self::TooLong => libnx_error(LibnxError::BadInput),
        }
    }
}

/// Static storage for parsed environment state.
///
/// Filled exactly once by the calling entry crate via the [`init_once`]
/// closure, then immutable. The fields are public so an entry crate in a
/// different crate can populate them inside that closure; the only `&mut`
/// path to an `EnvState` is `init_once`, so writes stay confined to the
/// one-shot initialization.
pub struct EnvState {
    /// True if running as NSO (system module), false if NRO (homebrew)
    pub is_nso: bool,

    /// Heap override (address, size)
    pub heap_override: Option<(NonNull<c_void>, usize)>,

    /// Argv string pointer
    pub argv: Option<NonNull<c_char>>,

    /// Thread and process handles
    pub main_thread_handle: Option<ThreadHandle>,
    pub process_handle: Option<ProcessHandle>,

    /// Syscall availability hints (192 bits for SVCs 0x00-0xBF)
    pub syscall_hints: SyscallHints,

    /// Random seed data
    pub random_seed: Option<[u64; 2]>,

    /// Last load result
    pub last_load_result: u32,

    /// Loader info string (pointer, size)
    pub loader_info: Option<(NonNull<c_char>, u64)>,

    /// User ID storage pointer
    pub user_id_storage: Option<NonNull<AccountUid>>,

    /// Where to write the program to run after this one, when the loader
    /// accepts one
    pub next_load: Option<NextLoad>,

    /// Service override entries from loader
    pub service_overrides: [Option<ServiceOverride>; MAX_SERVICE_OVERRIDES],
    pub service_override_count: usize,

    /// Applet type from loader
    pub applet_type: AppletType,

    /// APT workaround flag (true if APT is broken and should not be used)
    pub applet_workaround: bool,
}

impl EnvState {
    const fn new() -> Self {
        Self {
            is_nso: false,
            heap_override: None,
            argv: None,
            main_thread_handle: None,
            process_handle: None,
            syscall_hints: SyscallHints::new(),
            random_seed: None,
            last_load_result: 0,
            loader_info: None,
            user_id_storage: None,
            next_load: None,
            service_overrides: [None; MAX_SERVICE_OVERRIDES],
            service_override_count: 0,
            applet_type: AppletType::Default,
            applet_workaround: false,
        }
    }
}

/// A service override entry (name + handle)
#[derive(Debug, Clone, Copy)]
pub struct ServiceOverride {
    pub name: ServiceName,
    pub handle: ServiceHandle,
}

impl ServiceOverride {
    /// Create a new service override entry.
    pub const fn new(name: ServiceName, handle: ServiceHandle) -> Self {
        Self { name, handle }
    }
}

/// The 128-bit account id the loader writes a selected user into
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AccountUid {
    pub uid: [u64; 2],
}

/// What role the process was launched in.
///
/// The discriminants are the wire values the startup source uses and the
/// C-facing surface publishes, which is why the representation is pinned.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppletType {
    /// Default/unset
    Default = -1,
    /// Regular application
    Application = 0,
    /// System applet
    SystemApplet = 1,
    /// Library applet
    LibraryApplet = 2,
    /// Overlay applet
    OverlayApplet = 3,
    /// System application
    SystemApplication = 4,
}

impl AppletType {
    /// Applet flags: ApplicationOverride bit
    const FLAG_APPLICATION_OVERRIDE: u64 = 1 << 0;

    /// Create from raw loader values, applying flags
    pub const fn from_raw(value: u32, flags: u64) -> Self {
        let mut applet_type = match value {
            0 => Self::Application,
            1 => Self::SystemApplet,
            2 => Self::LibraryApplet,
            3 => Self::OverlayApplet,
            4 => Self::SystemApplication,
            _ => Self::Default,
        };

        // Apply ApplicationOverride flag if applicable
        if (flags & Self::FLAG_APPLICATION_OVERRIDE) != 0
            && matches!(applet_type, Self::SystemApplication)
        {
            applet_type = Self::Application;
        }

        applet_type
    }

    /// The wire value, at the enum's own representation.
    ///
    /// Signed, because [`AppletType::Default`] is `-1`: an unsigned result
    /// would make that one variant `0xFFFF_FFFF` and every caller that wanted
    /// a number back would have to undo the reinterpretation.
    pub const fn as_raw(self) -> i32 {
        self as i32
    }
}

/// Holds the loader's return address as a function rather than as a pointer.
///
/// An `AtomicPtr` would store it as a data pointer, and handing it back would
/// mean reinterpreting that pointer as a function, which the language permits
/// only through `transmute`. Keeping the value at its own type means it is
/// never reinterpreted at all. The lock costs a pair of uncontended atomics on
/// a path taken twice in the lifetime of a process.
struct ExitFunc {
    lock: Mutex,
    func: UnsafeCell<LoaderReturnFn>,
}

impl ExitFunc {
    const fn new() -> Self {
        Self {
            lock: Mutex::new(),
            func: UnsafeCell::new(None),
        }
    }

    /// Records where to return to, replacing whatever was there.
    fn set(&self, func: LoaderReturnFn) {
        self.lock.lock();
        // SAFETY: the lock is held, so no other thread holds a reference to
        // the cell for the duration of the write.
        unsafe { *self.func.get() = func };
        self.lock.unlock();
    }

    /// Reads back what was recorded, or `None` if nothing was.
    fn get(&self) -> LoaderReturnFn {
        self.lock.lock();
        // SAFETY: the lock is held, so no writer can be running; the value is
        // a plain function pointer, so the copy leaves nothing borrowed.
        let func = unsafe { *self.func.get() };
        self.lock.unlock();
        func
    }
}

// SAFETY: the only path to the `UnsafeCell` is through `set`/`get`, both of
// which hold `lock` for the whole of their access, so no two threads touch the
// cell at once and no reference outlives the critical section.
unsafe impl Sync for ExitFunc {}

/// Names the same role to the Applet Manager client.
///
/// Total in both directions: the startup parse folds anything it does not
/// recognise into [`AppletType::Default`], so every value that reaches here is
/// one the client also names. There is no "unknown role" for a caller to
/// handle, and the exhaustive match below is what keeps that true as either
/// side gains variants.
#[cfg(feature = "service-applet")]
impl From<AppletType> for nx_service_applet::AppletType {
    fn from(applet_type: AppletType) -> Self {
        match applet_type {
            AppletType::Default => Self::Default,
            AppletType::Application => Self::Application,
            AppletType::SystemApplet => Self::SystemApplet,
            AppletType::LibraryApplet => Self::LibraryApplet,
            AppletType::OverlayApplet => Self::OverlayApplet,
            AppletType::SystemApplication => Self::SystemApplication,
        }
    }
}

/// Global environment state wrapped in UnsafeCell for interior mutability
struct EnvStateWrapper(UnsafeCell<EnvState>);

impl EnvStateWrapper {
    const fn new() -> Self {
        Self(UnsafeCell::new(EnvState::new()))
    }

    /// Get mutable access to the environment state
    ///
    /// # Safety
    ///
    /// The returned pointer grants a one-shot `&mut EnvState`. It is sound to
    /// dereference only from inside `init_once`'s `Once::call_once`, which
    /// guarantees exactly one writer with no concurrent readers. `init_once`
    /// is the sole caller; the [`EnvState`] is read-only afterwards.
    unsafe fn get(&self) -> *mut EnvState {
        // SAFETY: Caller dereferences this only within Once::call_once.
        self.0.get()
    }

    /// Get immutable access to the environment state
    ///
    /// # Safety
    ///
    /// Caller must ensure the state has been initialized via init_once()
    /// before calling this method.
    unsafe fn get_ref(&self) -> &EnvState {
        // SAFETY: Caller guarantees initialization has completed
        unsafe { &*self.0.get() }
    }
}

unsafe impl Sync for EnvStateWrapper {}

/// The buffers the loader reads a chain-load request out of.
///
/// They belong to the loader, not to this process: it keeps running after this
/// program exits, and what it finds here is what it runs next. A program that
/// never asks leaves them as the loader left them, which is what makes not
/// asking mean "go back to where I came from".
#[derive(Debug, Clone, Copy)]
pub struct NextLoad {
    /// Where the path of the next program goes.
    path: LoaderBuffer,
    /// Where its command line goes.
    argv: LoaderBuffer,
}

impl NextLoad {
    /// How much the loader's path buffer holds, terminator included.
    ///
    /// The loader announces the buffers without saying how large they are, so
    /// their sizes are part of the convention rather than of the message. These
    /// are the ones every loader in this family allocates.
    pub const PATH_CAPACITY: usize = 512;

    /// How much the loader's command-line buffer holds, terminator included.
    pub const ARGV_CAPACITY: usize = 2048;

    /// Takes the two buffers out of the loader's startup configuration.
    ///
    /// This is the one place the loader's word is taken for anything: every
    /// write below rests on the guarantee stated here.
    ///
    /// # Safety
    ///
    /// Both pointers come from the loader's own configuration, where each
    /// addresses a buffer of at least the capacity documented above, writable
    /// and live for as long as this program runs.
    pub unsafe fn from_loader(path: NonNull<c_char>, argv: NonNull<c_char>) -> Self {
        Self {
            path: LoaderBuffer {
                ptr: path.cast(),
                capacity: Self::PATH_CAPACITY,
            },
            argv: LoaderBuffer {
                ptr: argv.cast(),
                capacity: Self::ARGV_CAPACITY,
            },
        }
    }

    /// Records what the loader should run next, in the order it reads it.
    fn write(self, path: &CStr, argv: &CStr) -> Result<(), SetNextLoadError> {
        // The loader reads the path to decide whether to run anything at all,
        // so the path lands last: until it does, whatever sits in the
        // command-line buffer is not part of any request. That ordering is what
        // keeps a request that does not fit from being acted on in part, with
        // no need to measure both halves before writing either.
        if !self.argv.write(argv.to_bytes_with_nul()) || !self.path.write(path.to_bytes_with_nul())
        {
            // An earlier request must not be left standing with a command line
            // that is no longer its own. Emptying the path withdraws it.
            self.path.clear();
            return Err(SetNextLoadError::TooLong);
        }

        Ok(())
    }
}

/// One of the loader's buffers: where it starts, and how much it takes.
///
/// The two travel together because neither is usable without the other, and a
/// caller holding them separately could pair a buffer with the capacity of its
/// neighbour.
#[derive(Debug, Clone, Copy)]
struct LoaderBuffer {
    ptr: NonNull<u8>,
    capacity: usize,
}

impl LoaderBuffer {
    /// Copies `bytes` in, or reports that they do not fit.
    ///
    /// Nothing is written when they do not: a truncated path names a different
    /// program, and the loader would run it without complaint.
    #[must_use = "bytes that did not fit were not written"]
    fn write(self, bytes: &[u8]) -> bool {
        if bytes.len() > self.capacity {
            return false;
        }

        // SAFETY: `NextLoad::from_loader` establishes that `ptr` addresses
        // `capacity` writable bytes for as long as this program runs, and the
        // check above keeps the copy inside them. The loader's buffers are its
        // own, so they cannot overlap the caller's `bytes`.
        unsafe { ptr::copy_nonoverlapping(bytes.as_ptr(), self.ptr.as_ptr(), bytes.len()) };
        true
    }

    /// Empties the buffer, withdrawing whatever it held.
    fn clear(self) {
        // A lone terminator fits in any buffer the loader hands over, so this
        // cannot be a write that does not fit.
        let _ = self.write(&[0]);
    }
}
