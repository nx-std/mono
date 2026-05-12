//! # nx-service-bsd
//!
//! Rust port of libnx's `bsd` BSD-socket service. The crate exposes:
//!
//! - [`connect_with_options`] / [`ConnectError`] — establishes a `bsd:u` or
//!   `bsd:s` session, wires up the transfer-memory the service requires, runs
//!   the `RegisterClient` + `StartMonitoring` handshake, and builds a pool of
//!   cloned sessions so socket calls can proceed concurrently.
//! - [`BsdService`] — Rust-facing API; one method per supported BSD command.
//! - Per-command error enums in [`cmif`] (re-exported here) — every fallible
//!   call surfaces three distinct sources: IPC send failure, CMIF parse
//!   failure, and the POSIX-domain errno reported by the service.
//!
//! This crate is the **Rust API only**. The FFI surface (`__nx_service_bsd__*`
//! symbols, `bsd_override.ld`, `nx-std` re-export) is intentionally deferred
//! to a follow-up PR.

#![no_std]

// `alloc` is needed for the session pool's `Box<[Session]>`.
extern crate alloc;
// `nx_panic_handler` provides `#[panic_handler]`.
extern crate nx_panic_handler as _;

use alloc::{boxed::Box, vec::Vec};

use nx_service_sm::SmService;
use nx_sf::{ServiceName, service::Session};
use nx_svc::mem::tmem::MemoryPermission;
use nx_sys_mem::tmem::{self, TransferMemoryBacking};

mod cmif;
mod fd;
mod proto;
mod session;
mod types;

pub use crate::{
    cmif::{
        AcceptError, BindError, CloseError, ConnectError as CmifConnectError, FcntlError,
        GetPeerNameError, GetSockNameError, GetSockOptError, IoctlError, ListenError, PollError,
        ReadError, RecvError, RecvFromError, RegisterClientError, SelectError, SelectTimeout,
        SendError, SendToError, SetSockOptError, ShutdownError, SocketError, StartMonitoringError,
        WriteError,
    },
    fd::BsdSockFd,
    proto::{SERVICE_NAME_SYSTEM, SERVICE_NAME_USER},
    types::{BsdConfig, BsdServiceType, ConnectOptions},
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

// SAFETY: every field is either an immutable kernel handle wrapper (`Session`,
// `TransferMemoryBacking`) or a `nx_std_sync::Mutex` / `Condvar` based pool.
// Concurrent IPC calls from different threads acquire distinct pool slots, so
// no thread-unsafe mutation is performed via shared `&self`.
unsafe impl Send for BsdService {}
unsafe impl Sync for BsdService {}

impl BsdService {
    /// `bsdSocket` (cmd 2). Creates a new socket and returns its descriptor.
    pub fn socket(&self, domain: i32, type_: i32, protocol: i32) -> Result<BsdSockFd, SocketError> {
        let g = self.pool.acquire();
        cmif::socket(g.session(), domain, type_, protocol)
    }

    /// `bsdBind` (cmd 13). `addr` is interpreted by the service as a `sockaddr`.
    pub fn bind(&self, sockfd: BsdSockFd, addr: &[u8]) -> Result<(), BindError> {
        let g = self.pool.acquire();
        cmif::bind(g.session(), sockfd, addr)
    }

    /// `bsdConnect` (cmd 14).
    pub fn connect(&self, sockfd: BsdSockFd, addr: &[u8]) -> Result<(), CmifConnectError> {
        let g = self.pool.acquire();
        cmif::connect(g.session(), sockfd, addr)
    }

    /// `bsdListen` (cmd 18).
    pub fn listen(&self, sockfd: BsdSockFd, backlog: i32) -> Result<(), ListenError> {
        let g = self.pool.acquire();
        cmif::listen(g.session(), sockfd, backlog)
    }

    /// `bsdAccept` (cmd 12). Returns the new descriptor and the actual length
    /// the service wrote into `addr_buf` (`socklen_t`).
    pub fn accept(
        &self,
        sockfd: BsdSockFd,
        addr_buf: &mut [u8],
    ) -> Result<(BsdSockFd, u32), AcceptError> {
        let g = self.pool.acquire();
        cmif::accept(g.session(), sockfd, addr_buf)
    }

    /// `bsdGetSockName` (cmd 16). Returns the `socklen_t` actually written.
    pub fn get_sock_name(
        &self,
        sockfd: BsdSockFd,
        addr_buf: &mut [u8],
    ) -> Result<u32, GetSockNameError> {
        let g = self.pool.acquire();
        cmif::get_sock_name(g.session(), sockfd, addr_buf)
    }

    /// `bsdGetPeerName` (cmd 15). Returns the `socklen_t` actually written.
    pub fn get_peer_name(
        &self,
        sockfd: BsdSockFd,
        addr_buf: &mut [u8],
    ) -> Result<u32, GetPeerNameError> {
        let g = self.pool.acquire();
        cmif::get_peer_name(g.session(), sockfd, addr_buf)
    }

    /// `bsdShutdown` (cmd 22).
    pub fn shutdown(&self, sockfd: BsdSockFd, how: i32) -> Result<(), ShutdownError> {
        let g = self.pool.acquire();
        cmif::shutdown(g.session(), sockfd, how)
    }

    /// `bsdRecv` (cmd 8). Returns the number of bytes written into `buf`.
    pub fn recv(&self, sockfd: BsdSockFd, buf: &mut [u8], flags: i32) -> Result<usize, RecvError> {
        let g = self.pool.acquire();
        cmif::recv(g.session(), sockfd, buf, flags)
    }

    /// `bsdRecvFrom` (cmd 9). Returns `(bytes_received, actual_src_addr_len)`.
    pub fn recv_from(
        &self,
        sockfd: BsdSockFd,
        buf: &mut [u8],
        flags: i32,
        src_addr: &mut [u8],
    ) -> Result<(usize, u32), RecvFromError> {
        let g = self.pool.acquire();
        cmif::recv_from(g.session(), sockfd, buf, flags, src_addr)
    }

    /// `bsdSend` (cmd 10). Returns the number of bytes the service accepted.
    pub fn send(&self, sockfd: BsdSockFd, buf: &[u8], flags: i32) -> Result<usize, SendError> {
        let g = self.pool.acquire();
        cmif::send(g.session(), sockfd, buf, flags)
    }

    /// `bsdSendTo` (cmd 11).
    pub fn send_to(
        &self,
        sockfd: BsdSockFd,
        buf: &[u8],
        flags: i32,
        dest_addr: &[u8],
    ) -> Result<usize, SendToError> {
        let g = self.pool.acquire();
        cmif::send_to(g.session(), sockfd, buf, flags, dest_addr)
    }

    /// `bsdRead` (cmd 25).
    pub fn read(&self, fd: BsdSockFd, buf: &mut [u8]) -> Result<usize, ReadError> {
        let g = self.pool.acquire();
        cmif::read(g.session(), fd, buf)
    }

    /// `bsdWrite` (cmd 24).
    pub fn write(&self, fd: BsdSockFd, buf: &[u8]) -> Result<usize, WriteError> {
        let g = self.pool.acquire();
        cmif::write(g.session(), fd, buf)
    }

    /// `bsdGetSockOpt` (cmd 17). Returns the actual `socklen_t` written.
    pub fn get_sock_opt(
        &self,
        sockfd: BsdSockFd,
        level: i32,
        optname: i32,
        optval: &mut [u8],
    ) -> Result<u32, GetSockOptError> {
        let g = self.pool.acquire();
        cmif::get_sock_opt(g.session(), sockfd, level, optname, optval)
    }

    /// `bsdSetSockOpt` (cmd 21).
    pub fn set_sock_opt(
        &self,
        sockfd: BsdSockFd,
        level: i32,
        optname: i32,
        optval: &[u8],
    ) -> Result<(), SetSockOptError> {
        let g = self.pool.acquire();
        cmif::set_sock_opt(g.session(), sockfd, level, optname, optval)
    }

    /// `bsdFcntl` (cmd 20). The BSD service supports only `F_GETFL` / `F_SETFL`.
    pub fn fcntl(&self, fd: BsdSockFd, cmd: i32, flags: i32) -> Result<i32, FcntlError> {
        let g = self.pool.acquire();
        cmif::fcntl(g.session(), fd, cmd, flags)
    }

    /// `bsdIoctl` (cmd 19) — generic case only.
    ///
    /// The special `SIOCGIFCONF` / `SIOCGIFMEDIA` / `SIOCGIFXMEDIA` variants
    /// (which interpret `data` to discover sub-buffers) are not implemented yet.
    pub fn ioctl(&self, fd: BsdSockFd, request: i32, data: &mut [u8]) -> Result<i32, IoctlError> {
        let g = self.pool.acquire();
        cmif::ioctl(g.session(), fd, request, data)
    }

    /// `bsdSelect` (cmd 5). Each `fd_set` slice carries the libnx `fd_set` byte
    /// layout; pass empty slices for fd_sets the caller does not need.
    pub fn select(
        &self,
        nfds: i32,
        readfds: &mut [u8],
        writefds: &mut [u8],
        exceptfds: &mut [u8],
        timeout: Option<SelectTimeout>,
    ) -> Result<i32, SelectError> {
        let g = self.pool.acquire();
        cmif::select(g.session(), nfds, readfds, writefds, exceptfds, timeout)
    }

    /// `bsdPoll` (cmd 6). `fds` carries the libnx `pollfd` array byte layout.
    pub fn poll(&self, fds: &mut [u8], nfds: u64, timeout: i32) -> Result<i32, PollError> {
        let g = self.pool.acquire();
        cmif::poll(g.session(), fds, nfds, timeout)
    }

    /// `bsdClose` (cmd 26). Consumes the descriptor to make double-close hard.
    pub fn close_fd(&self, fd: BsdSockFd) -> Result<(), CloseError> {
        let g = self.pool.acquire();
        cmif::close(g.session(), fd)
    }

    /// Tears down the service connection.
    ///
    /// Drops every pool session and the monitor session (each closes via
    /// `Drop`), then waits for the kernel to reclaim transfer-memory
    /// permissions before freeing the backing buffer. Mirrors libnx's
    /// `bsdExit`.
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

/// Establishes a `bsd:u`/`bsd:s` session, completes the libnx handshake, and
/// builds the session pool.
///
/// Steps mirror libnx's `_bsdInitialize`:
/// 1. Look up the main service handle via SM (with `Auto` fallback when set).
/// 2. Look up the monitor service handle via SM (same service name).
/// 3. Allocate the transfer memory the service requires.
/// 4. Send `RegisterClient` (cmd 0) on the main handle.
/// 5. Send `StartMonitoring` (cmd 1) on the monitor handle with the returned PID.
/// 6. Close the local copy of the tmem handle (the service keeps its own copy).
/// 7. Clone `num_sessions - 1` extra sessions and build the pool — slot 0 is
///    the original handle.
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
    let num_sessions = opts
        .config
        .num_sessions
        .clamp(1, session::MAX_SESSIONS as u32) as usize;
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
fn make_service(handle: nx_svc::ipc::Handle) -> Session {
    Session::from_handle(handle, 0)
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

    /// `IBsdServices::RegisterClient` (cmd 0) failed. The wrapped error
    /// distinguishes IPC-send / response-parse failures.
    #[error("failed to register bsd client")]
    RegisterClient(#[source] RegisterClientError),

    /// `IBsdServices::StartMonitoring` (cmd 1) failed.
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
