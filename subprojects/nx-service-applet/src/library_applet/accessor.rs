//! `ILibraryAppletAccessor` commands.
//!
//! One accessor drives one launched library applet: start it, wait for it to
//! exit, and exchange storages with it.

use nx_sf::{
    error::{
        GENERIC_ERROR,
        ToResultCode,
    },
    service::{
        DispatchError,
        DomainObject,
        OutHandleAttr,
    },
};
use nx_svc::{
    error::{
        ResultCode,
        ToResultCode as _,
    },
    sync::EventHandle,
};

use super::{
    storage::Storage,
    support::reanchor_object,
};
use crate::proto::{
    CMD_LAA_GET_APPLET_STATE_CHANGED_EVENT,
    CMD_LAA_GET_RESULT,
    CMD_LAA_POP_OUT_DATA,
    CMD_LAA_PUSH_IN_DATA,
    CMD_LAA_START,
    LibraryAppletExitReason,
};

/// An `ILibraryAppletAccessor` for one launched library applet.
///
/// Closes the server-side object on drop, which is what tears the applet down.
#[derive(Debug)]
pub struct LibraryAppletAccessor<'d> {
    object: DomainObject<'d>,
}

impl<'d> LibraryAppletAccessor<'d> {
    /// Wraps the domain object this accessor is addressed through.
    pub(super) fn new(object: DomainObject<'d>) -> Self {
        Self { object }
    }

    /// Gets the event signalled when the applet's state changes (cmd 0).
    ///
    /// # Autoclear semantics
    ///
    /// The kernel event is configured with `autoclear = false`, matching libnx's
    /// `_appletCmdGetEvent(..., autoclear=false, ...)`. A caller that waits on it
    /// more than once must reset the signal itself, or every later wait returns
    /// immediately.
    pub fn get_applet_state_changed_event(
        &self,
    ) -> Result<EventHandle, GetAppletStateChangedEventError> {
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

        let result = self
            .object
            .dispatch(CMD_LAA_GET_APPLET_STATE_CHANGED_EVENT)
            .out_handle(0, OutHandleAttr::Copy)
            .send(&mut buf)
            .map_err(GetAppletStateChangedEventError::Dispatch)?;

        if result.copy_handles.is_empty() {
            return Err(GetAppletStateChangedEventError::MissingHandle);
        }

        // SAFETY: Kernel returned a valid event handle in the response.
        Ok(EventHandle::from_raw_unchecked(result.copy_handles[0]))
    }

    /// Starts the applet (cmd 10).
    ///
    /// Returns as soon as the system accepts the request; the applet runs
    /// concurrently until [`join`](Self::join).
    pub fn start(&self) -> Result<(), StartError> {
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

        self.object
            .dispatch(CMD_LAA_START)
            .send(&mut buf)
            .map_err(StartError::Dispatch)?;

        Ok(())
    }

    /// Reads the applet's exit status (cmd 30).
    ///
    /// A success reply means the applet exited normally; a service error is the
    /// applet's own result, which [`LibraryAppletExitReason`] classifies.
    pub fn get_result(&self) -> Result<LibraryAppletExitReason, GetResultError> {
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

        match self.object.dispatch(CMD_LAA_GET_RESULT).send(&mut buf) {
            Ok(_) => Ok(LibraryAppletExitReason::Normal),
            Err(DispatchError::ParseResponse(nx_sf::cmif::ParseError::ServiceError(code))) => {
                Ok(LibraryAppletExitReason::from_result_code(code))
            }
            Err(err) => Err(GetResultError::Dispatch(err)),
        }
    }

    /// Waits for the applet to exit and reports why (cmd 30 after a wait).
    ///
    /// Blocks until the user dismisses the applet, so this must not be called
    /// from a context that cannot block indefinitely.
    pub fn join(&self, event: &EventHandle) -> Result<LibraryAppletExitReason, JoinError> {
        // SAFETY: `event` is a live handle this accessor obtained from the
        // server, and the wait borrows it for the call's duration.
        unsafe { nx_svc::sync::wait_synchronization_single(event, u64::MAX) }
            .map_err(JoinError::Wait)?;

        self.get_result().map_err(JoinError::GetResult)
    }

    /// Pushes `storage` to the applet as an input storage (cmd 100).
    ///
    /// The server copies the contents, so `storage` remains ours to close.
    /// Order matters: every library applet reads its common arguments as the
    /// first storage pushed.
    pub fn push_in_data(&self, storage: &Storage<'_>) -> Result<(), PushInDataError> {
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

        self.object
            .dispatch(CMD_LAA_PUSH_IN_DATA)
            .in_object(storage.object_id())
            .send(&mut buf)
            .map_err(PushInDataError::Dispatch)?;

        Ok(())
    }

    /// Pops the applet's reply storage (cmd 101).
    pub fn pop_out_data(&self) -> Result<Storage<'d>, PopOutDataError> {
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

        let mut result = self
            .object
            .dispatch(CMD_LAA_POP_OUT_DATA)
            .out_objects(1)
            .send(&mut buf)
            .map_err(PopOutDataError::Dispatch)?;

        let raw_object_id = result
            .take_object(0)
            .ok_or(PopOutDataError::MissingObject)?
            .into_raw_object_id();

        Ok(Storage::new(reanchor_object(
            self.object.domain(),
            raw_object_id,
        )))
    }
}

/// Error returned by [`LibraryAppletAccessor::get_applet_state_changed_event`].
#[derive(Debug, thiserror::Error)]
pub enum GetAppletStateChangedEventError {
    /// Failed to dispatch the request.
    #[error("failed to dispatch request")]
    Dispatch(#[source] DispatchError),
    /// Response did not contain the expected handle.
    #[error("missing handle in response")]
    MissingHandle,
}

impl ToResultCode for GetAppletStateChangedEventError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::Dispatch(err) => err.to_rc(),
            // Rejected locally after a successful reply, so no server
            // named a code for it.
            Self::MissingHandle => GENERIC_ERROR,
        }
    }
}

/// Error returned by [`LibraryAppletAccessor::start`].
#[derive(Debug, thiserror::Error)]
pub enum StartError {
    /// Failed to dispatch the request.
    #[error("failed to dispatch request")]
    Dispatch(#[source] DispatchError),
}

impl ToResultCode for StartError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::Dispatch(err) => err.to_rc(),
        }
    }
}

/// Error returned by [`LibraryAppletAccessor::get_result`].
#[derive(Debug, thiserror::Error)]
pub enum GetResultError {
    /// Failed to dispatch the request.
    #[error("failed to dispatch request")]
    Dispatch(#[source] DispatchError),
}

impl ToResultCode for GetResultError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::Dispatch(err) => err.to_rc(),
        }
    }
}

/// Error returned by [`LibraryAppletAccessor::join`].
#[derive(Debug, thiserror::Error)]
pub enum JoinError {
    /// Failed to wait on the state-changed event.
    #[error("failed to wait for the applet to exit")]
    Wait(#[source] nx_svc::sync::WaitSyncError),
    /// Failed to read the applet's exit status.
    #[error("failed to read the applet result")]
    GetResult(#[source] GetResultError),
}

impl ToResultCode for JoinError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::Wait(err) => err.to_rc(),
            Self::GetResult(err) => err.to_rc(),
        }
    }
}

/// Error returned by [`LibraryAppletAccessor::push_in_data`].
#[derive(Debug, thiserror::Error)]
pub enum PushInDataError {
    /// Failed to dispatch the request.
    #[error("failed to dispatch request")]
    Dispatch(#[source] DispatchError),
}

impl ToResultCode for PushInDataError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::Dispatch(err) => err.to_rc(),
        }
    }
}

/// Error returned by [`LibraryAppletAccessor::pop_out_data`].
#[derive(Debug, thiserror::Error)]
pub enum PopOutDataError {
    /// Failed to dispatch the request.
    #[error("failed to dispatch request")]
    Dispatch(#[source] DispatchError),
    /// Response did not carry the reply storage.
    #[error("missing storage object in response")]
    MissingObject,
}

impl ToResultCode for PopOutDataError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::Dispatch(err) => err.to_rc(),
            // Rejected locally after a successful reply, so no server
            // named a code for it.
            Self::MissingObject => GENERIC_ERROR,
        }
    }
}
