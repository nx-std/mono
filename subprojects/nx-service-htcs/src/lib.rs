//! HTC sockets (`htcs`) service implementation.
//!
//! Provides access to the HTCS service for host-target communication
//! (socket-like IPC between the Switch and a connected development host).
//!
//! ## Architecture
//!
//! - **`htcs`** (manager): Domain-mode root session with a session pool for
//!   concurrent dispatch. [`connect_cmif`] obtains the manager session,
//!   converts it to a domain, performs PID init, and creates the pool.
//!   [`HtcsService::create_socket`] returns [`HtcsSocket`] sub-objects.
//!
//! - **`htcs`** (monitor): Separate non-domain session that receives PID
//!   init (cmd 101) during connection. Kept alive for the service lifetime
//!   but has no user-facing commands.
//!
//! ## Divergence from libnx
//!
//! libnx's `htcs.c` keeps guarded global singletons (`g_htcsSrv`,
//! `g_htcsMonitor`) managed by `NX_GENERATE_SERVICE_GUARD`, with a
//! `SessionMgr` for concurrent domain dispatch. This crate follows the
//! convention of the other `nx-service-*` crates: connect once via
//! [`connect_cmif`], then call methods directly.
//!
//! The session pool size is caller-configurable (libnx's
//! `HTCS_SESSION_COUNT_MAX` is 0x10).

#![no_std]

extern crate alloc;
extern crate nx_panic_handler as _; // provides #[panic_handler]

use alloc::{boxed::Box, vec::Vec};

use nx_service_sm::SmService;
use nx_sf::service::{ConvertToDomainError, DispatchError, Domain, Session, clone_current_object};

mod cmif;
mod proto;
mod session;
pub mod types;

pub use nx_sf::service::DispatchError as IpcDispatchError;

use crate::session::SessionPool;
pub use crate::{
    cmif::{
        AcceptResultsError, AcceptStartError, CreateSocketError, RecvStartError, SendStartError,
        StartRecvError, StartSelectError, StartSendError,
    },
    proto::SERVICE_NAME,
    types::{
        AcceptResultData, ContinueSendResult, EndSelectResult, FD_SET_SIZE, HtcsAddressFamily,
        HtcsFcntlFlag, HtcsFcntlOperation, HtcsFdSet, HtcsMessageFlag, HtcsPeerName, HtcsPortName,
        HtcsShutdownType, HtcsSockAddr, HtcsSocketError, HtcsTimeVal, PEER_NAME_MAX, PORT_NAME_MAX,
        SESSION_COUNT_MAX, SOCKET_COUNT_MAX, SocketResult, StartSendResult, TransferResult,
    },
};

/// Connected HTCS manager service wrapper.
///
/// Operates in domain mode with a session pool for concurrent IPC dispatch.
/// Socket sub-objects share the domain with the manager. Dropping the
/// service closes all pool sessions and the monitor session.
pub struct HtcsService {
    pool: SessionPool,
    #[allow(dead_code)]
    monitor: Session,
}

// SAFETY: every field is either an immutable kernel handle wrapper or a
// `nx_std_sync::Mutex` / `Condvar` based pool. The HOS kernel serializes
// `svcSendSyncRequest` per session handle; concurrent IPC calls acquire
// distinct pool slots.
unsafe impl Send for HtcsService {}
unsafe impl Sync for HtcsService {}

impl HtcsService {
    /// Gets the "any" peer name (cmd 10).
    pub fn get_peer_name_any(&self) -> Result<HtcsPeerName, DispatchError> {
        let guard = self.pool.acquire();
        cmif::get_peer_name(guard.domain(), proto::GET_PEER_NAME_ANY)
    }

    /// Gets the default host name (cmd 11).
    pub fn get_default_host_name(&self) -> Result<HtcsPeerName, DispatchError> {
        let guard = self.pool.acquire();
        cmif::get_peer_name(guard.domain(), proto::GET_DEFAULT_HOST_NAME)
    }

    /// Creates a new socket (cmd 13).
    ///
    /// Returns the HTCS-level error code and the new socket sub-object.
    pub fn create_socket(
        &self,
        enable_disconnection_emulation: bool,
    ) -> Result<(i32, HtcsSocket<'_>), CreateSocketError> {
        let guard = self.pool.acquire();
        let (err, object_id) = cmif::create_socket(guard.domain(), enable_disconnection_emulation)?;
        Ok((
            err,
            HtcsSocket {
                service: self,
                object_id,
            },
        ))
    }

    /// Starts a select operation (cmd 130).
    ///
    /// Returns `(task_id, event_handle)`.
    pub fn start_select(
        &self,
        tv: &HtcsTimeVal,
        read_fds: &[i32],
        write_fds: &[i32],
        except_fds: &[i32],
    ) -> Result<(u32, u32), StartSelectError> {
        let guard = self.pool.acquire();
        cmif::start_select(guard.domain(), tv, read_fds, write_fds, except_fds)
    }

    /// Ends a select operation (cmd 131).
    pub fn end_select(
        &self,
        task_id: u32,
        read_fds: &mut [i32],
        write_fds: &mut [i32],
        except_fds: &mut [i32],
    ) -> Result<EndSelectResult, DispatchError> {
        let guard = self.pool.acquire();
        let out = cmif::end_select(guard.domain(), task_id, read_fds, write_fds, except_fds)?;
        Ok(EndSelectResult {
            err: out.err,
            count: out.count,
        })
    }
}

/// HTCS socket sub-object obtained via [`HtcsService::create_socket`] or
/// [`HtcsSocket::accept_results`].
///
/// The lifetime parameter ties the socket to its parent service so the
/// underlying domain session outlives the sub-object.
pub struct HtcsSocket<'svc> {
    service: &'svc HtcsService,
    object_id: u32,
}

impl HtcsSocket<'_> {
    /// Closes the socket (cmd 0) and returns the HTCS-level error/result.
    ///
    /// Consumes the socket. After this call, the domain sub-object is
    /// released via `Drop` of the underlying [`DomainObject`].
    pub fn socket_close(self) -> Result<SocketResult, DispatchError> {
        let guard = self.service.pool.acquire();
        // SAFETY: `self.object_id` was returned by the server within this
        // service's domain table, and this method consumes the socket, so the
        // close obligation it takes on is discharged exactly once. The pool
        // guard makes this slot exclusive, so no other live `DomainObject`
        // addresses the same id concurrently.
        let object = guard
            .open_object_for_close_unchecked(self.object_id)
            .expect("socket object_id obtained from server");
        let result = cmif::socket_close(object.as_borrowed());
        // Dropping `object` sends the per-object close on the pool session.
        result
    }

    /// Releases the domain sub-object without issuing cmd 0.
    ///
    /// Use this when the socket was already closed via [`socket_close`](Self::socket_close),
    /// or when cleanup is all that's needed.
    pub fn close_handle(self) {
        let guard = self.service.pool.acquire();
        // SAFETY: same justification as `socket_close`; this method consumes
        // the socket too.
        let _object = guard
            .open_object_for_close_unchecked(self.object_id)
            .expect("socket object_id obtained from server");
        // Dropping `_object` sends the per-object close on the pool session.
    }

    /// Connects to an address (cmd 1).
    pub fn connect(&self, address: &HtcsSockAddr) -> Result<SocketResult, DispatchError> {
        let guard = self.service.pool.acquire();
        // SAFETY: `self.object_id` was returned by the server within this
        // service's domain table; the pool guard makes this slot exclusive,
        // so no other live `DomainObject` addresses the same id concurrently.
        let object = guard
            .open_object_unchecked(self.object_id)
            .expect("socket object_id obtained from server");
        cmif::socket_cmd_in_address(object, proto::SOCKET_CONNECT, address)
    }

    /// Binds to an address (cmd 2).
    pub fn bind(&self, address: &HtcsSockAddr) -> Result<SocketResult, DispatchError> {
        let guard = self.service.pool.acquire();
        // SAFETY: `self.object_id` was returned by the server within this
        // service's domain table; the pool guard makes this slot exclusive,
        // so no other live `DomainObject` addresses the same id concurrently.
        let object = guard
            .open_object_unchecked(self.object_id)
            .expect("socket object_id obtained from server");
        cmif::socket_cmd_in_address(object, proto::SOCKET_BIND, address)
    }

    /// Listens for connections (cmd 3).
    pub fn listen(&self, backlog: i32) -> Result<SocketResult, DispatchError> {
        let guard = self.service.pool.acquire();
        // SAFETY: `self.object_id` was returned by the server within this
        // service's domain table; the pool guard makes this slot exclusive,
        // so no other live `DomainObject` addresses the same id concurrently.
        let object = guard
            .open_object_unchecked(self.object_id)
            .expect("socket object_id obtained from server");
        cmif::socket_cmd_in_i32(object, proto::SOCKET_LISTEN, backlog)
    }

    /// Shuts down part of a connection (cmd 7).
    pub fn shutdown(&self, how: i32) -> Result<SocketResult, DispatchError> {
        let guard = self.service.pool.acquire();
        // SAFETY: `self.object_id` was returned by the server within this
        // service's domain table; the pool guard makes this slot exclusive,
        // so no other live `DomainObject` addresses the same id concurrently.
        let object = guard
            .open_object_unchecked(self.object_id)
            .expect("socket object_id obtained from server");
        cmif::socket_cmd_in_i32(object, proto::SOCKET_SHUTDOWN, how)
    }

    /// File control (cmd 8).
    pub fn fcntl(&self, command: i32, value: i32) -> Result<SocketResult, DispatchError> {
        let guard = self.service.pool.acquire();
        // SAFETY: `self.object_id` was returned by the server within this
        // service's domain table; the pool guard makes this slot exclusive,
        // so no other live `DomainObject` addresses the same id concurrently.
        let object = guard
            .open_object_unchecked(self.object_id)
            .expect("socket object_id obtained from server");
        cmif::socket_fcntl(object, command, value)
    }

    /// Starts an async accept operation (cmd 9).
    ///
    /// Returns `(task_id, event_handle)`.
    pub fn accept_start(&self) -> Result<(u32, u32), AcceptStartError> {
        let guard = self.service.pool.acquire();
        // SAFETY: `self.object_id` was returned by the server within this
        // service's domain table; the pool guard makes this slot exclusive,
        // so no other live `DomainObject` addresses the same id concurrently.
        let object = guard
            .open_object_unchecked(self.object_id)
            .expect("socket object_id obtained from server");
        cmif::socket_accept_start(object)
    }

    /// Gets the result of an async accept (cmd 10).
    ///
    /// Returns the accepted socket, the peer address, and the HTCS error code.
    pub fn accept_results(&self, task_id: u32) -> Result<AcceptResultData<'_>, AcceptResultsError> {
        let guard = self.service.pool.acquire();
        // SAFETY: `self.object_id` was returned by the server within this
        // service's domain table; the pool guard makes this slot exclusive,
        // so no other live `DomainObject` addresses the same id concurrently.
        let object = guard
            .open_object_unchecked(self.object_id)
            .expect("socket object_id obtained from server");
        let (out, socket_object_id) = cmif::socket_accept_results(object, task_id)?;
        Ok(AcceptResultData {
            err: out.err,
            address: out.address,
            socket: HtcsSocket {
                service: self.service,
                object_id: socket_object_id,
            },
        })
    }

    /// Starts an async recv operation (cmd 11).
    ///
    /// Returns `(task_id, event_handle)`.
    pub fn recv_start(&self, mem_size: i32, flags: i32) -> Result<(u32, u32), RecvStartError> {
        let guard = self.service.pool.acquire();
        // SAFETY: `self.object_id` was returned by the server within this
        // service's domain table; the pool guard makes this slot exclusive,
        // so no other live `DomainObject` addresses the same id concurrently.
        let object = guard
            .open_object_unchecked(self.object_id)
            .expect("socket object_id obtained from server");
        cmif::socket_recv_start(object, mem_size, flags)
    }

    /// Gets the result of an async recv (cmd 12).
    ///
    /// Data is written into `buffer`. Returns the transfer result with
    /// error code and actual byte count.
    pub fn recv_results(
        &self,
        task_id: u32,
        buffer: &mut [u8],
    ) -> Result<TransferResult, DispatchError> {
        let guard = self.service.pool.acquire();
        // SAFETY: `self.object_id` was returned by the server within this
        // service's domain table; the pool guard makes this slot exclusive,
        // so no other live `DomainObject` addresses the same id concurrently.
        let object = guard
            .open_object_unchecked(self.object_id)
            .expect("socket object_id obtained from server");
        cmif::socket_recv_results(object, task_id, buffer)
    }

    /// Gets the result of an async send (cmd 16).
    pub fn send_results(&self, task_id: u32) -> Result<TransferResult, DispatchError> {
        let guard = self.service.pool.acquire();
        // SAFETY: `self.object_id` was returned by the server within this
        // service's domain table; the pool guard makes this slot exclusive,
        // so no other live `DomainObject` addresses the same id concurrently.
        let object = guard
            .open_object_unchecked(self.object_id)
            .expect("socket object_id obtained from server");
        cmif::socket_cmd_in_u32_out_transfer(object, proto::SOCKET_SEND_RESULTS, task_id)
    }

    /// Starts a large-buffer send (cmd 17).
    ///
    /// For sending data larger than the IPC buffer. After start, use
    /// [`continue_send`](Self::continue_send) to feed data, then
    /// [`end_send`](Self::end_send) to finalize.
    pub fn start_send(&self, size: i64, flags: i32) -> Result<StartSendResult, StartSendError> {
        let guard = self.service.pool.acquire();
        // SAFETY: `self.object_id` was returned by the server within this
        // service's domain table; the pool guard makes this slot exclusive,
        // so no other live `DomainObject` addresses the same id concurrently.
        let object = guard
            .open_object_unchecked(self.object_id)
            .expect("socket object_id obtained from server");
        let (out, event_handle) = cmif::socket_start_send(object, size, flags)?;
        Ok(StartSendResult {
            task_id: out.task_id,
            event_handle,
            max_size: out.max_size,
        })
    }

    /// Ends a large-buffer send (cmd 19).
    pub fn end_send(&self, task_id: u32) -> Result<TransferResult, DispatchError> {
        let guard = self.service.pool.acquire();
        // SAFETY: `self.object_id` was returned by the server within this
        // service's domain table; the pool guard makes this slot exclusive,
        // so no other live `DomainObject` addresses the same id concurrently.
        let object = guard
            .open_object_unchecked(self.object_id)
            .expect("socket object_id obtained from server");
        cmif::socket_cmd_in_u32_out_transfer(object, proto::SOCKET_END_SEND, task_id)
    }

    /// Starts a large-buffer recv (cmd 20).
    ///
    /// Returns `(task_id, event_handle)`.
    pub fn start_recv(&self, size: i64, flags: i32) -> Result<(u32, u32), StartRecvError> {
        let guard = self.service.pool.acquire();
        // SAFETY: `self.object_id` was returned by the server within this
        // service's domain table; the pool guard makes this slot exclusive,
        // so no other live `DomainObject` addresses the same id concurrently.
        let object = guard
            .open_object_unchecked(self.object_id)
            .expect("socket object_id obtained from server");
        cmif::socket_start_recv(object, size, flags)
    }

    /// Ends a large-buffer recv (cmd 21).
    ///
    /// Data is written into `buffer`. Returns the transfer result.
    pub fn end_recv(
        &self,
        task_id: u32,
        buffer: &mut [u8],
    ) -> Result<TransferResult, DispatchError> {
        let guard = self.service.pool.acquire();
        // SAFETY: `self.object_id` was returned by the server within this
        // service's domain table; the pool guard makes this slot exclusive,
        // so no other live `DomainObject` addresses the same id concurrently.
        let object = guard
            .open_object_unchecked(self.object_id)
            .expect("socket object_id obtained from server");
        cmif::socket_end_recv(object, task_id, buffer)
    }

    /// Starts an async send with buffer (cmd 22).
    ///
    /// Returns `(task_id, event_handle)`.
    pub fn send_start(&self, buffer: &[u8], flags: i32) -> Result<(u32, u32), SendStartError> {
        let guard = self.service.pool.acquire();
        // SAFETY: `self.object_id` was returned by the server within this
        // service's domain table; the pool guard makes this slot exclusive,
        // so no other live `DomainObject` addresses the same id concurrently.
        let object = guard
            .open_object_unchecked(self.object_id)
            .expect("socket object_id obtained from server");
        cmif::socket_send_start(object, buffer, flags)
    }

    /// Continues a large-buffer send (cmd 23).
    pub fn continue_send(
        &self,
        task_id: u32,
        buffer: &[u8],
    ) -> Result<ContinueSendResult, DispatchError> {
        let guard = self.service.pool.acquire();
        // SAFETY: `self.object_id` was returned by the server within this
        // service's domain table; the pool guard makes this slot exclusive,
        // so no other live `DomainObject` addresses the same id concurrently.
        let object = guard
            .open_object_unchecked(self.object_id)
            .expect("socket object_id obtained from server");
        let out = cmif::socket_continue_send(object, task_id, buffer)?;
        Ok(ContinueSendResult {
            size: out.size,
            wait: out.wait != 0,
        })
    }

    /// Gets the underlying file descriptor (cmd 130).
    pub fn get_primitive(&self) -> Result<i32, DispatchError> {
        let guard = self.service.pool.acquire();
        // SAFETY: `self.object_id` was returned by the server within this
        // service's domain table; the pool guard makes this slot exclusive,
        // so no other live `DomainObject` addresses the same id concurrently.
        let object = guard
            .open_object_unchecked(self.object_id)
            .expect("socket object_id obtained from server");
        cmif::socket_get_primitive(object)
    }
}

/// Connects to the HTCS service using CMIF.
///
/// Sets up both the manager (domain-mode with session pool) and monitor
/// sessions. `num_sessions` controls the pool size for concurrent dispatch
/// (libnx default max is 0x10).
pub fn connect_cmif(sm: &SmService, num_sessions: usize) -> Result<HtcsService, ConnectCmifError> {
    let manager_handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError::GetService)?;

    let monitor_handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError::GetService)?;

    let manager_session = Session::open(manager_handle);
    let pointer_buffer_size = manager_session.pointer_buffer_size();

    let manager = manager_session
        .convert_to_domain()
        .map_err(|(_session, err)| ConnectCmifError::ConvertToDomain(err))?;

    let monitor = Session::new(monitor_handle, 0);

    // PID init on the domain manager (cmd 100).
    cmif::manager_pid_init(manager.as_borrowed()).map_err(ConnectCmifError::ManagerPidInit)?;

    // PID init on the non-domain monitor (cmd 101).
    cmif::monitor_pid_init(&monitor).map_err(ConnectCmifError::MonitorPidInit)?;

    // Build session pool from cloned domain sessions. The first slot owns
    // the root domain handle; the remaining slots are cloned domain handles
    // that share the same server-side object table.
    let mut sessions: Vec<Domain> = Vec::with_capacity(num_sessions);
    sessions.push(manager);
    for _ in 1..num_sessions {
        let cloned_handle =
            clone_current_object(sessions[0].handle()).map_err(ConnectCmifError::CloneSession)?;
        // SAFETY: Cloning a domain session yields another kernel handle addressing the same
        // domain object table on the server side.
        let cloned_domain = Domain::new_unchecked(cloned_handle, pointer_buffer_size);
        sessions.push(cloned_domain);
    }

    let pool = SessionPool::new(sessions.into_boxed_slice() as Box<[Domain]>);

    Ok(HtcsService { pool, monitor })
}

/// Errors returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectCmifError {
    /// SM lookup for `htcs` failed.
    #[error("failed to look up htcs service via sm")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
    /// Converting the manager session to a domain failed.
    #[error("failed to ConvertToDomain on htcs manager session")]
    ConvertToDomain(#[source] ConvertToDomainError),
    /// PID init on the manager session failed.
    #[error("failed to send PID init on htcs manager session")]
    ManagerPidInit(#[source] DispatchError),
    /// PID init on the monitor session failed.
    #[error("failed to send PID init on htcs monitor session")]
    MonitorPidInit(#[source] DispatchError),
    /// Cloning the manager session for the pool failed.
    #[error("failed to clone htcs session for the pool")]
    CloneSession(#[source] nx_sf::service::CloneObjectError),
}
