//! NV (NVIDIA Driver) Service Implementation.
//!
//! This crate provides access to the Nintendo Switch's NV service, which handles:
//! - Device management (open/close)
//! - Ioctl operations for GPU communication
//! - Event queries for synchronization
//!
//! The NV service is the foundation for GPU operations on the Nintendo Switch,
//! providing low-level access to NVIDIA hardware through a standardized interface.

#![no_std]

extern crate nx_panic_handler; // Provide #![panic_handler]

use nx_service_applet::{AppletType, aruid::Aruid};
use nx_service_sm::SmService;
use nx_sf::service::Service;
use nx_svc::{
    ipc::Handle as SessionHandle,
    mem::tmem::{Handle as TmemHandle, MemoryPermission},
    process::Handle as ProcessHandle,
    raw::Handle as RawHandle,
};
use nx_sys_mem::tmem::{self, TransferMemoryBacking};

mod cmif;
mod proto;
pub mod types;

pub use self::{
    cmif::{
        CloseError, InitializeError, Ioctl2Error, Ioctl3Error, IoctlError, OpenError,
        QueryEventError, SetClientPidError,
    },
    proto::{
        SERVICE_NAME_APPLET, SERVICE_NAME_APPLICATION, SERVICE_NAME_FACTORY, SERVICE_NAME_SYSTEM,
    },
    types::{
        CloseNvError, IoctlNvError, NV_IOC_NONE, NV_IOC_READ, NV_IOC_WRITE, NvConfig, NvEventId,
        NvServiceType, OpenNvError, QueryEventNvError, nv_event_id_ctrl_syncpt, nv_ioc_dir,
        nv_ioc_size,
    },
};

/// NV service session wrapper.
///
/// Provides access to NVIDIA driver operations including device management
/// and ioctl commands.
pub struct NvService {
    /// Main service session.
    main_session: Service,
    /// Clone session for parallel ioctl operations.
    clone_session: Service,
    /// Transfer memory backing for cleanup.
    ///
    /// The handle is closed early (after Initialize), but we keep the backing
    /// memory pointer for cleanup. This matches libnx's `tmemCloseHandle()`
    /// pattern.
    transfer_mem_backing: TransferMemoryBacking,
}

// SAFETY: NvService is safe to send across threads because:
// - All Service instances are just session handles (u32)
// - TransferMemoryBacking just holds a pointer and size, no kernel handle
unsafe impl Send for NvService {}

// SAFETY: NvService is safe to share across threads because:
// - All operations go through the kernel which handles synchronization
// - Ioctl operations may use either main or clone session based on request type
unsafe impl Sync for NvService {}

impl NvService {
    /// Returns the main service session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.main_session.session
    }

    /// Returns the clone service session handle.
    #[inline]
    pub fn clone_session(&self) -> SessionHandle {
        self.clone_session.session
    }

    /// Returns the appropriate session for a given ioctl request.
    ///
    /// Certain high-frequency ioctls are routed to the clone session to
    /// avoid contention on the main session.
    #[inline]
    fn session_for_request(&self, request: u32) -> SessionHandle {
        let masked = request & proto::IOCTL_MASK;

        // Check masked ioctls
        for &ioctl in proto::CLONE_SESSION_IOCTLS {
            if masked == ioctl {
                return self.clone_session.session;
            }
        }

        // Check exact match ioctls
        for &ioctl in proto::CLONE_SESSION_IOCTLS_EXACT {
            if request == ioctl {
                return self.clone_session.session;
            }
        }

        self.main_session.session
    }

    /// Opens a device by path.
    ///
    /// Returns the file descriptor on success.
    pub fn open(&self, device_path: &str) -> Result<u32, OpenError> {
        cmif::open(self.main_session.session, device_path.as_bytes())
    }

    /// Performs an ioctl operation.
    ///
    /// The `argp` buffer is used for both input and output based on the
    /// direction flags in the request code.
    pub fn ioctl(&self, fd: u32, request: u32, argp: &mut [u8]) -> Result<(), IoctlError> {
        let bufsize = nv_ioc_size(request);
        let dir = nv_ioc_dir(request);

        let in_size = if (dir & NV_IOC_WRITE) != 0 {
            bufsize
        } else {
            0
        };

        let out_size = if (dir & NV_IOC_READ) != 0 { bufsize } else { 0 };

        let session = self.session_for_request(request);

        cmif::ioctl(session, fd, request, in_size, out_size, argp.as_mut_ptr())
    }

    /// Performs an ioctl2 operation with an extra input buffer.
    ///
    /// Available on firmware 3.0.0+.
    pub fn ioctl2(
        &self,
        fd: u32,
        request: u32,
        argp: &mut [u8],
        inbuf: &[u8],
    ) -> Result<(), Ioctl2Error> {
        let bufsize = nv_ioc_size(request);
        let dir = nv_ioc_dir(request);

        let in_size = if (dir & NV_IOC_WRITE) != 0 {
            bufsize
        } else {
            0
        };

        let out_size = if (dir & NV_IOC_READ) != 0 { bufsize } else { 0 };

        let session = self.session_for_request(request);

        cmif::ioctl2(
            session,
            fd,
            request,
            in_size,
            out_size,
            argp.as_mut_ptr(),
            inbuf.as_ptr(),
            inbuf.len(),
        )
    }

    /// Performs an ioctl3 operation with an extra output buffer.
    ///
    /// Available on firmware 3.0.0+.
    pub fn ioctl3(
        &self,
        fd: u32,
        request: u32,
        argp: &mut [u8],
        outbuf: &mut [u8],
    ) -> Result<(), Ioctl3Error> {
        let bufsize = nv_ioc_size(request);
        let dir = nv_ioc_dir(request);

        let in_size = if (dir & NV_IOC_WRITE) != 0 {
            bufsize
        } else {
            0
        };

        let out_size = if (dir & NV_IOC_READ) != 0 { bufsize } else { 0 };

        let session = self.session_for_request(request);

        cmif::ioctl3(
            session,
            fd,
            request,
            in_size,
            out_size,
            argp.as_mut_ptr(),
            outbuf.as_mut_ptr(),
            outbuf.len(),
        )
    }

    /// Closes a device file descriptor.
    pub fn close_fd(&self, fd: u32) -> Result<(), CloseError> {
        cmif::close(self.main_session.session, fd)
    }

    /// Queries an event for a device.
    ///
    /// Returns the event handle on success.
    pub fn query_event(&self, fd: u32, event_id: u32) -> Result<RawHandle, QueryEventError> {
        cmif::query_event(self.main_session.session, fd, event_id)
    }

    /// Consumes and closes the NV service session.
    pub fn close(self) {
        // Close clone session first to match libnx behavior
        self.clone_session.close();
        self.main_session.close();

        // Wait for transfer memory permission to return to RW, then free backing.
        // The handle was already closed during connect(), so we just need to wait
        // for the service to release the memory and then free our backing allocation.
        if let Some(src) = self.transfer_mem_backing.src {
            let _ = unsafe {
                tmem::wait_for_permission_raw(
                    src,
                    self.transfer_mem_backing.perm,
                    MemoryPermission::RW,
                )
            };
        }

        unsafe { tmem::free_backing(self.transfer_mem_backing) };
    }
}

/// Connects to the NV service.
///
/// # Arguments
///
/// * `sm` - Service manager session
/// * `applet_type` - The current applet type (used to resolve service type if `Auto`)
/// * `aruid` - Applet resource user ID, or `None` if not available
/// * `config` - Configuration options for the NV service
///
/// # Returns
///
/// A connected [`NvService`] instance on success.
pub fn connect(
    sm: &SmService,
    applet_type: AppletType,
    aruid: Option<Aruid>,
    config: NvConfig,
) -> Result<NvService, ConnectError> {
    // Determine service type
    let service_type = if config.service_type == NvServiceType::Auto {
        resolve_service_type(applet_type)
    } else {
        config.service_type
    };

    // Get service name based on type
    let service_name = match service_type {
        NvServiceType::Auto => unreachable!("Auto should have been resolved"),
        NvServiceType::Application => SERVICE_NAME_APPLICATION,
        NvServiceType::Applet => SERVICE_NAME_APPLET,
        NvServiceType::System => SERVICE_NAME_SYSTEM,
        NvServiceType::Factory => SERVICE_NAME_FACTORY,
    };

    // Get service handle from SM
    let handle = sm
        .get_service_handle_cmif(service_name)
        .map_err(ConnectError::GetService)?;

    let main_session = Service {
        session: handle,
        own_handle: 1,
        object_id: 0,
        pointer_buffer_size: 0,
    };

    // Create transfer memory
    let transfer_mem = unsafe { tmem::create(config.transfer_mem_size, MemoryPermission::NONE) }
        .map_err(ConnectError::CreateTransferMemory)?;

    // Initialize the service
    // SAFETY: We're converting our tmem handle to the expected type for the IPC call.
    // The handle is valid because we just created the transfer memory above.
    if let Err(e) = cmif::initialize(
        main_session.session,
        ProcessHandle::current_process(),
        unsafe { TmemHandle::from_raw(transfer_mem.handle().to_raw()) },
        config.transfer_mem_size as u32,
    ) {
        // Clean up on failure
        let _ = unsafe { tmem::close(transfer_mem) };
        main_session.close();
        return Err(ConnectError::Initialize(e));
    }

    // Close the tmem handle early, matching libnx's tmemCloseHandle() pattern.
    // The service has its own copy of the handle from Initialize().
    // We keep the backing memory pointer for cleanup in close().
    let transfer_mem_backing = match unsafe { tmem::close_handle_keep_backing(transfer_mem) } {
        Ok(backing) => backing,
        Err(e) => {
            // If closing the handle fails, free the backing and clean up
            unsafe { tmem::free_backing(e.backing) };
            main_session.close();
            return Err(ConnectError::CloseTransferMemHandle(e.reason));
        }
    };

    // Clone the session for parallel ioctl operations
    let clone_session = match main_session.try_clone_ex(1) {
        Ok(s) => s,
        Err(e) => {
            main_session.close();
            unsafe { tmem::free_backing(transfer_mem_backing) };
            return Err(ConnectError::CloneSession(e));
        }
    };

    // Try to set client PID (best effort, may not have ARUID)
    if let Some(aruid) = aruid {
        // Ignore errors - matches libnx behavior
        let _ = cmif::set_client_pid(main_session.session, aruid);
    }

    Ok(NvService {
        main_session,
        clone_session,
        transfer_mem_backing,
    })
}

/// Resolves the automatic service type based on applet type.
fn resolve_service_type(applet_type: AppletType) -> NvServiceType {
    match applet_type {
        AppletType::None => NvServiceType::System,
        AppletType::Default | AppletType::Application | AppletType::SystemApplication => {
            NvServiceType::Application
        }
        AppletType::SystemApplet | AppletType::LibraryApplet | AppletType::OverlayApplet => {
            NvServiceType::Applet
        }
    }
}

/// Error returned by [`connect`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    /// Failed to get service handle from SM.
    #[error("failed to get service")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
    /// Failed to create transfer memory.
    #[error("failed to create transfer memory")]
    CreateTransferMemory(#[source] tmem::CreateError),
    /// Failed to initialize service.
    #[error("failed to initialize service")]
    Initialize(#[source] InitializeError),
    /// Failed to close transfer memory handle.
    #[error("failed to close transfer memory handle")]
    CloseTransferMemHandle(#[source] nx_svc::mem::tmem::CloseHandleError),
    /// Failed to clone session.
    #[error("failed to clone session")]
    CloneSession(#[source] nx_sf::service::TryCloneExError),
}
