//! # nx-service-bsd
//!
//! An IPC client for the Horizon `bsd` socket service. The crate exposes:
//!
//! - [`connect_with_options`] / [`ConnectError`] — establishes a `bsd:u` or
//!   `bsd:s` session, wires up the transfer-memory the service requires, runs
//!   the `RegisterClient` + `StartMonitoring` handshake, and builds a pool of
//!   cloned sessions so socket calls can proceed concurrently.
//! - [`BsdService`] — Rust-facing API; one method per command in the
//!   interface.
//! - [`CommandError`] — how every one of those commands fails, naming the
//!   [`Command`] that failed and, for a command the service rejected, the
//!   [`PosixError`] it reported.
//!
//! ## What this crate is, and is not
//!
//! It is the IPC client and nothing above it. Commands take the descriptors
//! and byte buffers the service exchanges; they do not construct socket
//! addresses, interpret option values, or hold a process-wide session. Those
//! belong to the socket layer above, which is also where the C-facing surface
//! will live.
//!
//! That boundary is why a rejected command surfaces a [`PosixError`] rather
//! than a number: the service answers in Linux error numbering, which is not
//! what a C `errno` slot on this platform holds, and only a layer that knows
//! which numbering its caller reads can produce one. See [`posix`].
//!
//! This crate is the **Rust API only**. The FFI surface (`__nx_service_bsd__*`
//! symbols, `bsd_override.ld`, `nx-std` re-export) is intentionally deferred
//! to a follow-up PR.

#![no_std]

// `alloc` is needed for the session pool's `Box<[Session]>`.
extern crate alloc;
// `nx_panic_handler` provides `#[panic_handler]`.
extern crate nx_panic_handler as _;

use alloc::{
    boxed::Box,
    vec::Vec,
};

use nx_service_sm::SmService;
use nx_sf::{
    ServiceName,
    service::{
        OwnedSessionHandle,
        Session,
    },
};
use nx_std_path::Path;
use nx_svc::mem::tmem::MemoryPermission;
use nx_sys_mem::tmem::{
    self,
    TransferMemoryBacking,
};

mod cmif;
pub mod config;
mod fd;
pub mod posix;
mod proto;
mod session;
pub mod sockaddr;
pub mod transfer;

pub use crate::{
    cmif::{
        CommandError,
        FcntlOp,
        RecvTimeout,
        RegisterClientError,
        SelectTimeout,
        StartMonitoringError,
    },
    config::{
        BsdConfig,
        BsdServiceType,
        BufferEfficiency,
        BufferEfficiencyError,
        ConfigVersion,
        ConnectOptions,
        SessionCount,
        SessionCountError,
    },
    fd::BsdSockFd,
    posix::PosixError,
    proto::{
        Command,
        SERVICE_NAME_SYSTEM,
        SERVICE_NAME_USER,
    },
    sockaddr::{
        AddrTooLongError,
        RawSockAddr,
    },
    transfer::{
        RecvFlags,
        SendFlags,
        Shutdown,
        StatusFlags,
    },
};

/// Owns a connected BSD socket service.
///
/// Co-locates the session pool, the dedicated monitor session, and the
/// transfer-memory backing because all three share a single lifecycle:
/// established by [`connect_with_options`] and released by [`Self::close`].
/// Splitting them would not eliminate any cross-concern coupling — every
/// teardown path touches every field.
pub struct BsdService {
    pool: session::SessionPool,
    monitor_session: Session,
    transfer_mem_backing: TransferMemoryBacking,
}

// SAFETY: what keeps this type off the auto traits is the raw backing pointer
// inside `TransferMemoryBacking`, which is never dereferenced while the
// service is live - it is read only by `close`, which consumes `self` and so
// runs on one thread with no other reference outstanding. Everything else is
// either an immutable kernel handle wrapper (`Session`) or the pool, which is
// guarded by a `nx_std_sync` mutex and condvar; concurrent commands take
// distinct pool slots, so no thread-unsafe mutation happens through `&self`.
unsafe impl Send for BsdService {}
unsafe impl Sync for BsdService {}

impl BsdService {
    /// Creates a socket and returns its descriptor.
    /// # Errors
    ///
    /// [`CommandError::Service`] when the service refuses to create the socket:
    /// [`PosixError::ProtocolNotSupported`] or
    /// [`PosixError::AddressFamilyNotSupported`] for a combination the service does
    /// not implement, [`PosixError::ProcessFdLimit`] when this client holds no free
    /// descriptors.
    pub fn socket(
        &self,
        domain: i32,
        type_: i32,
        protocol: i32,
    ) -> Result<BsdSockFd, CommandError> {
        let g = self.pool.acquire();
        cmif::socket(g.session(), domain, type_, protocol)
    }

    /// Creates a socket exempt from the system's socket accounting.
    ///
    /// Identical to [`Self::socket`] in every other respect.
    /// # Errors
    ///
    /// As [`Self::socket`], plus [`PosixError::PermissionDenied`] when this client
    /// may not create exempt sockets.
    pub fn socket_exempt(
        &self,
        domain: i32,
        type_: i32,
        protocol: i32,
    ) -> Result<BsdSockFd, CommandError> {
        let g = self.pool.acquire();
        cmif::socket_exempt(g.session(), domain, type_, protocol)
    }

    /// Opens a path in the service's own namespace.
    /// # Errors
    ///
    /// [`CommandError::Service`] carrying [`PosixError::NotFound`] for a path the
    /// service does not know, or [`PosixError::PermissionDenied`] for one it will
    /// not open to this client.
    pub fn open(&self, path: &Path, flags: i32) -> Result<BsdSockFd, CommandError> {
        let g = self.pool.acquire();
        cmif::open(g.session(), path, flags)
    }

    /// Assigns a local address to a socket.
    /// # Errors
    ///
    /// [`CommandError::Service`] carrying [`PosixError::AddressInUse`] when another
    /// socket already holds the address, or [`PosixError::AddressNotAvailable`]
    /// when it belongs to no local interface.
    pub fn bind(&self, sockfd: BsdSockFd, addr: &RawSockAddr) -> Result<(), CommandError> {
        let g = self.pool.acquire();
        cmif::bind(g.session(), sockfd, addr)
    }

    /// Initiates a connection to the peer named by `addr`.
    /// # Errors
    ///
    /// [`CommandError::Service`] carrying [`PosixError::InProgress`] on a
    /// non-blocking socket whose handshake has started — the caller waits for
    /// writability rather than treating it as a failure — or
    /// [`PosixError::ConnectionRefused`], [`PosixError::TimedOut`],
    /// [`PosixError::NetworkUnreachable`] when the peer cannot be reached.
    pub fn connect(&self, sockfd: BsdSockFd, addr: &RawSockAddr) -> Result<(), CommandError> {
        let g = self.pool.acquire();
        cmif::connect(g.session(), sockfd, addr)
    }

    /// Marks a socket as accepting connections.
    /// # Errors
    ///
    /// [`CommandError::Service`] carrying [`PosixError::InvalidArgument`] when the
    /// socket is not one that can accept connections.
    pub fn listen(&self, sockfd: BsdSockFd, backlog: i32) -> Result<(), CommandError> {
        let g = self.pool.acquire();
        cmif::listen(g.session(), sockfd, backlog)
    }

    /// Takes the next connection off a listening socket's queue.
    ///
    /// Returns the new descriptor and the peer's address.
    /// # Errors
    ///
    /// [`CommandError::Service`] carrying [`PosixError::WouldBlock`] when the socket
    /// is non-blocking and no connection is queued, or
    /// [`PosixError::ConnectionAborted`] when the queued connection died before it
    /// could be handed over.
    pub fn accept(&self, sockfd: BsdSockFd) -> Result<(BsdSockFd, RawSockAddr), CommandError> {
        let g = self.pool.acquire();
        cmif::accept(g.session(), sockfd)
    }

    /// Reports the socket's own address.
    /// # Errors
    ///
    /// [`CommandError::Service`] carrying [`PosixError::BadFd`] for a descriptor the
    /// service does not recognise.
    pub fn get_sock_name(&self, sockfd: BsdSockFd) -> Result<RawSockAddr, CommandError> {
        let g = self.pool.acquire();
        cmif::get_sock_name(g.session(), sockfd)
    }

    /// Reports the connected peer's address.
    /// # Errors
    ///
    /// [`CommandError::Service`] carrying [`PosixError::NotConnected`] when the
    /// socket has no peer.
    pub fn get_peer_name(&self, sockfd: BsdSockFd) -> Result<RawSockAddr, CommandError> {
        let g = self.pool.acquire();
        cmif::get_peer_name(g.session(), sockfd)
    }

    /// Disables further sends, receives, or both, on one socket.
    /// # Errors
    ///
    /// [`CommandError::Service`] carrying [`PosixError::NotConnected`] when the
    /// socket has nothing left to shut down.
    pub fn shutdown(&self, sockfd: BsdSockFd, how: Shutdown) -> Result<(), CommandError> {
        let g = self.pool.acquire();
        cmif::shutdown(g.session(), sockfd, how)
    }

    /// Applies `how` to every socket this client owns.
    ///
    /// # Errors
    ///
    /// [`CommandError::Service`] when the service refuses the request. There
    /// is no argument left for it to object to: [`Shutdown`] can only name a
    /// direction the interface implements.
    pub fn shutdown_all_sockets(&self, how: Shutdown) -> Result<(), CommandError> {
        let g = self.pool.acquire();
        cmif::shutdown_all_sockets(g.session(), how)
    }

    /// Receives from a connected socket, returning the byte count.
    /// # Errors
    ///
    /// [`CommandError::Service`] carrying [`PosixError::WouldBlock`] when the socket
    /// is non-blocking and nothing has arrived, or
    /// [`PosixError::ConnectionReset`]/[`PosixError::TimedOut`] when the connection
    /// ended under it. A closed connection is a zero byte count, not an error.
    pub fn recv(
        &self,
        sockfd: BsdSockFd,
        buf: &mut [u8],
        flags: RecvFlags,
    ) -> Result<usize, CommandError> {
        let g = self.pool.acquire();
        cmif::recv(g.session(), sockfd, buf, flags)
    }

    /// Receives and reports the sender's address.
    /// # Errors
    ///
    /// As [`Self::recv`].
    pub fn recv_from(
        &self,
        sockfd: BsdSockFd,
        buf: &mut [u8],
        flags: RecvFlags,
    ) -> Result<(usize, RawSockAddr), CommandError> {
        let g = self.pool.acquire();
        cmif::recv_from(g.session(), sockfd, buf, flags)
    }

    /// Sends on a connected socket, returning the accepted byte count.
    /// # Errors
    ///
    /// [`CommandError::Service`] carrying [`PosixError::WouldBlock`] when the send
    /// buffer is full on a non-blocking socket, [`PosixError::BrokenPipe`] or
    /// [`PosixError::ConnectionReset`] when the peer is gone, or
    /// [`PosixError::MessageTooLong`] for a datagram larger than the path allows.
    pub fn send(
        &self,
        sockfd: BsdSockFd,
        buf: &[u8],
        flags: SendFlags,
    ) -> Result<usize, CommandError> {
        let g = self.pool.acquire();
        cmif::send(g.session(), sockfd, buf, flags)
    }

    /// Sends to an explicit address, returning the accepted byte count.
    /// # Errors
    ///
    /// As [`Self::send`], plus [`PosixError::DestinationAddressRequired`] when
    /// `dest_addr` names no destination.
    pub fn send_to(
        &self,
        sockfd: BsdSockFd,
        buf: &[u8],
        flags: SendFlags,
        dest_addr: &RawSockAddr,
    ) -> Result<usize, CommandError> {
        let g = self.pool.acquire();
        cmif::send_to(g.session(), sockfd, buf, flags, dest_addr)
    }

    /// Receives up to `vlen` messages in one request.
    ///
    /// `buf` carries the caller's `mmsghdr` array. Requires `[7.0.0+]`; on
    /// older firmware the service rejects the command.
    /// # Errors
    ///
    /// [`CommandError::Service`] carrying [`PosixError::OperationNotSupported`] on
    /// firmware older than `[7.0.0]`, otherwise as [`Self::recv`].
    pub fn recv_mmsg(
        &self,
        sockfd: BsdSockFd,
        buf: &mut [u8],
        vlen: i32,
        flags: RecvFlags,
        timeout: RecvTimeout,
    ) -> Result<i32, CommandError> {
        let g = self.pool.acquire();
        cmif::recv_mmsg(g.session(), sockfd, buf, vlen, flags, timeout)
    }

    /// Sends up to `vlen` messages in one request.
    ///
    /// `buf` carries the caller's `mmsghdr` array and is written back with
    /// each message's accepted length. Requires `[7.0.0+]`; on older firmware
    /// the service rejects the command.
    /// # Errors
    ///
    /// [`CommandError::Service`] carrying [`PosixError::OperationNotSupported`] on
    /// firmware older than `[7.0.0]`, otherwise as [`Self::send`].
    pub fn send_mmsg(
        &self,
        sockfd: BsdSockFd,
        buf: &mut [u8],
        vlen: i32,
        flags: SendFlags,
    ) -> Result<i32, CommandError> {
        let g = self.pool.acquire();
        cmif::send_mmsg(g.session(), sockfd, buf, vlen, flags)
    }

    /// Reads from a descriptor, returning the byte count.
    /// # Errors
    ///
    /// As [`Self::recv`].
    pub fn read(&self, fd: BsdSockFd, buf: &mut [u8]) -> Result<usize, CommandError> {
        let g = self.pool.acquire();
        cmif::read(g.session(), fd, buf)
    }

    /// Writes to a descriptor, returning the accepted byte count.
    /// # Errors
    ///
    /// As [`Self::send`].
    pub fn write(&self, fd: BsdSockFd, buf: &[u8]) -> Result<usize, CommandError> {
        let g = self.pool.acquire();
        cmif::write(g.session(), fd, buf)
    }

    /// Reads a socket option.
    ///
    /// `T` is the option's own type, which is what says how many bytes it
    /// occupies; nothing is left for a caller to size. Asking for the wrong
    /// `T` reads whatever the service wrote into a differently-sized buffer,
    /// so the caller is the one that has to pair the option with its type.
    ///
    /// # Errors
    ///
    /// [`CommandError::Service`] carrying [`PosixError::ProtocolNotAvailable`]
    /// for an option the level does not define, or
    /// [`PosixError::InvalidArgument`] when `T` is smaller than the option's
    /// value.
    pub fn get_sock_opt<T>(
        &self,
        sockfd: BsdSockFd,
        level: i32,
        optname: i32,
    ) -> Result<T, CommandError>
    where
        T: zerocopy::FromBytes + zerocopy::Immutable + zerocopy::IntoBytes,
    {
        let g = self.pool.acquire();
        cmif::get_sock_opt(g.session(), sockfd, level, optname)
    }

    /// Reads a socket option into `optval`, returning how many bytes it holds.
    ///
    /// The untyped counterpart of [`Self::get_sock_opt`], for a caller that already has the
    /// buffer: a C surface is handed one, along with the length its caller chose, by somebody who
    /// has already decided which option they mean.
    ///
    /// # Errors
    ///
    /// As [`Self::get_sock_opt`].
    pub fn get_sock_opt_bytes(
        &self,
        sockfd: BsdSockFd,
        level: i32,
        optname: i32,
        optval: &mut [u8],
    ) -> Result<usize, CommandError> {
        let g = self.pool.acquire();
        cmif::get_sock_opt_bytes(g.session(), sockfd, level, optname, optval)
    }

    /// Writes a socket option from `optval`.
    ///
    /// The untyped counterpart of [`Self::set_sock_opt`], for the reason given on
    /// [`Self::get_sock_opt_bytes`].
    ///
    /// # Errors
    ///
    /// As [`Self::get_sock_opt`].
    pub fn set_sock_opt_bytes(
        &self,
        sockfd: BsdSockFd,
        level: i32,
        optname: i32,
        optval: &[u8],
    ) -> Result<(), CommandError> {
        let g = self.pool.acquire();
        cmif::set_sock_opt_bytes(g.session(), sockfd, level, optname, optval)
    }

    /// Writes a socket option.
    /// # Errors
    ///
    /// As [`Self::get_sock_opt`].
    pub fn set_sock_opt<T>(
        &self,
        sockfd: BsdSockFd,
        level: i32,
        optname: i32,
        optval: &T,
    ) -> Result<(), CommandError>
    where
        T: zerocopy::Immutable + zerocopy::IntoBytes,
    {
        let g = self.pool.acquire();
        cmif::set_sock_opt(g.session(), sockfd, level, optname, optval)
    }

    /// Reads or replaces a descriptor's status flags.
    ///
    /// Returns the descriptor's flags for [`FcntlOp::GetFlags`], and an empty
    /// set for [`FcntlOp::SetFlags`].
    /// # Errors
    ///
    /// [`CommandError::Service`] carrying [`PosixError::BadFd`] for a descriptor the
    /// service does not recognise.
    pub fn fcntl(&self, fd: BsdSockFd, op: FcntlOp) -> Result<StatusFlags, CommandError> {
        let g = self.pool.acquire();
        cmif::fcntl(g.session(), fd, op)
    }

    /// Issues a device control request whose payload is one flat block.
    ///
    /// The requests that answer with a header plus a list go through
    /// [`Self::ioctl_with_entries`].
    /// # Errors
    ///
    /// [`CommandError::Service`] carrying [`PosixError::InvalidArgument`] for a
    /// request the descriptor does not answer, or
    /// [`PosixError::OperationNotSupported`] for one the service does not implement.
    pub fn ioctl(&self, fd: BsdSockFd, request: i32, data: &mut [u8]) -> Result<i32, CommandError> {
        let g = self.pool.acquire();
        cmif::ioctl(g.session(), fd, request, data)
    }

    /// Issues a device control request that answers with a header plus a list.
    ///
    /// The shape `SIOCGIFCONF`, `SIOCGIFMEDIA` and `SIOCGIFXMEDIA` take: the
    /// caller passes the header and the list separately rather than embedding
    /// a pointer to the second in the first.
    /// # Errors
    ///
    /// As [`Self::ioctl`].
    pub fn ioctl_with_entries(
        &self,
        fd: BsdSockFd,
        request: i32,
        header: &mut [u8],
        entries: &mut [u8],
    ) -> Result<i32, CommandError> {
        let g = self.pool.acquire();
        cmif::ioctl_with_entries(g.session(), fd, request, header, entries)
    }

    /// Reads or writes a kernel networking parameter.
    ///
    /// `name` is the MIB naming the parameter, `new_value` is what to write
    /// (empty for a plain read), and `old_value` receives the previous value.
    /// Returns the length written into `old_value`.
    /// # Errors
    ///
    /// [`CommandError::Service`] carrying [`PosixError::NotFound`] for a MIB naming
    /// no parameter, or [`PosixError::InvalidArgument`] when `old_value` is too
    /// short to hold it.
    pub fn sysctl(
        &self,
        name: &[i32],
        new_value: &[u8],
        old_value: &mut [u8],
    ) -> Result<u64, CommandError> {
        let g = self.pool.acquire();
        cmif::sysctl(g.session(), name, new_value, old_value)
    }

    /// Waits for readiness across three descriptor sets.
    ///
    /// Each `fd_set` slice carries the C `fd_set` byte layout; pass empty
    /// slices for fd_sets the caller does not need.
    /// # Errors
    ///
    /// [`CommandError::Service`] carrying [`PosixError::InvalidArgument`] for an
    /// `nfds` the fd_sets do not cover, or [`PosixError::Interrupted`] when the wait
    /// was broken off. A timeout that expires is a zero count, not an error.
    pub fn select(
        &self,
        nfds: i32,
        readfds: &mut [u8],
        writefds: &mut [u8],
        exceptfds: &mut [u8],
        timeout: Option<SelectTimeout>,
    ) -> Result<i32, CommandError> {
        let g = self.pool.acquire();
        cmif::select(g.session(), nfds, readfds, writefds, exceptfds, timeout)
    }

    /// Waits for readiness across a descriptor array.
    ///
    /// `fds` carries the C `pollfd` array byte layout.
    /// # Errors
    ///
    /// As [`Self::select`].
    pub fn poll(&self, fds: &mut [u8], nfds: u64, timeout: i32) -> Result<i32, CommandError> {
        let g = self.pool.acquire();
        cmif::poll(g.session(), fds, nfds, timeout)
    }

    /// Produces a second descriptor naming the same socket.
    /// # Errors
    ///
    /// [`CommandError::Service`] carrying [`PosixError::ProcessFdLimit`] when this
    /// client holds no free descriptors.
    pub fn duplicate_socket(&self, sockfd: BsdSockFd) -> Result<BsdSockFd, CommandError> {
        let g = self.pool.acquire();
        cmif::duplicate_socket(g.session(), sockfd)
    }

    /// Releases a descriptor.
    ///
    /// Takes the descriptor by value. That does not prevent a second close —
    /// [`BsdSockFd`] is `Copy`, and a descriptor is only a number — but it is
    /// what lets the layer above hold the descriptor in a type that is not,
    /// and have this be the one call that consumes it.
    /// # Errors
    ///
    /// [`CommandError::Service`] carrying [`PosixError::BadFd`] for a descriptor the
    /// service does not recognise, which is what a double close looks like.
    pub fn close_fd(&self, fd: BsdSockFd) -> Result<(), CommandError> {
        let g = self.pool.acquire();
        cmif::close(g.session(), fd)
    }

    /// Tears down the service connection.
    ///
    /// Drops every pool session and the monitor session (each closes via
    /// `Drop`), then waits for the kernel to reclaim transfer-memory
    /// permissions before freeing the backing buffer.
    pub fn close(self) {
        let BsdService {
            pool,
            monitor_session,
            transfer_mem_backing,
        } = self;
        // Close every pooled session, then the monitor session, before
        // waiting for the kernel to release the transfer memory.
        drop(pool);
        drop(monitor_session);

        if let Some(src) = transfer_mem_backing.src {
            // SAFETY: `src` was supplied by `close_handle_keep_backing`, which
            // owns the allocation. We block until the kernel transitions the
            // mapping back to RW (i.e. the service released it) before freeing.
            let _ = unsafe {
                tmem::wait_for_permission_raw(src, transfer_mem_backing.perm, MemoryPermission::RW)
            };
        }
        // SAFETY: the backing has not been freed by any other path and the
        // wait_for_permission_raw call above ensured no kernel reference remains.
        unsafe { tmem::free_backing(transfer_mem_backing) };
    }
}

/// Establishes a `bsd:u`/`bsd:s` session, completes the service handshake,
/// and builds the session pool.
///
/// The handshake is:
/// 1. Look up the main service handle via SM (with `Auto` fallback when set).
/// 2. Look up the monitor service handle via SM (same service name).
/// 3. Allocate the transfer memory the service requires.
/// 4. Send `RegisterClient` on the main handle.
/// 5. Send `StartMonitoring` on the monitor handle with the returned PID.
/// 6. Close the local copy of the tmem handle (the service keeps its own copy).
/// 7. Clone `num_sessions - 1` extra sessions and build the pool — slot 0 is
///    the original handle.
///
/// # Errors
///
/// One [`ConnectError`] variant per step above, so a failure names the stage
/// it stopped at. Every variant is returned with the resources acquired so far
/// already released: no session, transfer memory, or kernel handle outlives a
/// failed connect.
pub fn connect_with_options(
    sm: &SmService,
    opts: &ConnectOptions,
) -> Result<BsdService, ConnectError> {
    // 1. Pick the service name list to try in order.
    let candidates: &[ServiceName] = match opts.service_type {
        BsdServiceType::Auto => &[SERVICE_NAME_SYSTEM, SERVICE_NAME_USER],
        BsdServiceType::System => &[SERVICE_NAME_SYSTEM],
        BsdServiceType::User => &[SERVICE_NAME_USER],
    };

    let (main, chosen_name) = open_main_service(sm, candidates)?;

    // 2. Open the monitor session on the same service name.
    // `main` is dropped (closing its session) automatically if this `?` fails.
    let monitor_handle = sm
        .get_service_handle_cmif(chosen_name)
        .map_err(ConnectError::GetMonitorService)?;
    let monitor = make_service(monitor_handle);

    // 3. Allocate transfer memory.
    let tmem_size = opts.config.transfer_mem_size();
    // SAFETY: we just computed a non-zero size when defaults are in play; if
    // the caller supplied a degenerate config that yields zero, the kernel
    // will reject the create and we propagate the error.
    let transfer_mem = match unsafe { tmem::create(tmem_size, MemoryPermission::NONE) } {
        Ok(tm) => tm,
        Err(err) => {
            // `monitor` and `main` are dropped at scope exit, closing both sessions.
            return Err(ConnectError::CreateTransferMemory(err));
        }
    };

    // 4. RegisterClient on main. The tmem handle accessor is gated behind the
    // `ffi` feature of `nx-sys-mem`, which we enable in `Cargo.toml`.
    let tmem_handle = transfer_mem.handle();
    let pid =
        match cmif::register_client(main.handle(), &opts.config, tmem_handle, tmem_size as u64) {
            Ok(pid) => pid,
            Err(err) => {
                // SAFETY: we own the tmem object and it has not been registered.
                let _ = unsafe { tmem::close(transfer_mem) };
                // `monitor` and `main` close via `Drop` at scope exit.
                return Err(ConnectError::RegisterClient(err));
            }
        };

    // 5. StartMonitoring on monitor.
    if let Err(err) = cmif::start_monitoring(monitor.handle(), pid) {
        // SAFETY: tmem object is still owned by us; release it cleanly.
        let _ = unsafe { tmem::close(transfer_mem) };
        // `monitor` and `main` close via `Drop` at scope exit.
        return Err(ConnectError::StartMonitoring(err));
    }

    // 6. Close the tmem handle locally, keeping the backing for later cleanup.
    // SAFETY: `transfer_mem` is the value we created and registered above; the
    // service now holds an independent copy of the handle.
    let transfer_mem_backing = match unsafe { tmem::close_handle_keep_backing(transfer_mem) } {
        Ok(backing) => backing,
        Err(err) => {
            // SAFETY: the kernel-side handle is already invalid (or in an
            // error state); we still own the backing allocation.
            unsafe { tmem::free_backing(err.backing) };
            // `monitor` and `main` close via `Drop` at scope exit.
            return Err(ConnectError::CloseTmemHandle(err.reason));
        }
    };

    // 7. Build the pool: slot 0 is the main session, the rest are clones.
    // `SessionCount` is bounded to what the pool's free-mask can track, so
    // there is nothing to clamp here.
    let num_sessions = opts.config.num_sessions.to_len();
    let mut sessions: Vec<Session> = Vec::with_capacity(num_sessions);
    sessions.push(main);
    for _ in 1..num_sessions {
        // Borrow slot 0 (the original main) to drive each clone.
        let parent = &sessions[0];
        match parent.try_clone() {
            Ok(clone) => sessions.push(clone),
            Err(err) => {
                // Tear down: drop all already-collected sessions (each closes
                // via `Drop`), drop the monitor, and free the tmem backing.
                drop(sessions);
                drop(monitor);
                // SAFETY: backing was just produced by close_handle_keep_backing
                // and has not been freed elsewhere.
                unsafe { tmem::free_backing(transfer_mem_backing) };
                return Err(ConnectError::CloneSession(err));
            }
        }
    }

    let pool = session::SessionPool::new(sessions.into_boxed_slice() as Box<[Session]>);

    Ok(BsdService {
        pool,
        monitor_session: monitor,
        transfer_mem_backing,
    })
}

/// Tries each candidate service name in order, returning the first that succeeds.
fn open_main_service(
    sm: &SmService,
    candidates: &[ServiceName],
) -> Result<(Session, ServiceName), ConnectError> {
    let mut last_err = None;
    for name in candidates {
        match sm.get_service_handle_cmif(*name) {
            Ok(handle) => return Ok((make_service(handle), *name)),
            Err(err) => last_err = Some(err),
        }
    }
    // Unwrap is fine here: `candidates` is non-empty by construction, so we
    // always recorded at least one error before falling through.
    Err(ConnectError::GetService(
        last_err.expect("candidates is non-empty by construction"),
    ))
}

/// Wraps a raw session handle in a `Session` with default metadata. Mirrors
/// what NV does — pointer-buffer-size is left at 0 so every auto-select buffer
/// is transmitted as a MapAlias buffer, which the BSD server accepts.
fn make_service(handle: OwnedSessionHandle) -> Session {
    Session::new(handle, 0)
}

/// Errors returned by [`connect_with_options`]. One variant per distinct source.
#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    /// Looking up the main BSD service handle via SM failed for every
    /// candidate name. The wrapped error is the last failure encountered.
    ///
    /// Possible causes:
    /// - Neither `bsd:s` nor `bsd:u` is registered (system not yet ready,
    ///   wrong context).
    /// - Permission denied for the requested variant.
    #[error("failed to look up bsd service via sm")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),

    /// The main service was acquired but the monitor service (same name) could
    /// not be looked up via SM.
    #[error("failed to look up bsd monitor service via sm")]
    GetMonitorService(#[source] nx_service_sm::GetServiceCmifError),

    /// `svcCreateTransferMemory` failed — usually an out-of-memory condition
    /// or an invalid (zero) size derived from the config.
    #[error("failed to create transfer memory")]
    CreateTransferMemory(#[source] tmem::CreateError),

    /// `IBsdServices::RegisterClient` failed. The wrapped error
    /// distinguishes IPC-send / response-parse failures.
    #[error("failed to register bsd client")]
    RegisterClient(#[source] RegisterClientError),

    /// `IBsdServices::StartMonitoring` failed.
    #[error("failed to start bsd monitoring")]
    StartMonitoring(#[source] StartMonitoringError),

    /// Closing the local transfer-memory handle after `RegisterClient`
    /// completed succeeded for the service but failed locally.
    #[error("failed to close transfer memory handle")]
    CloseTmemHandle(#[source] nx_svc::mem::tmem::CloseHandleError),

    /// Cloning the main session to fill the session pool failed.
    #[error("failed to clone bsd session")]
    CloneSession(#[source] nx_sf::service::CloneObjectError),
}
