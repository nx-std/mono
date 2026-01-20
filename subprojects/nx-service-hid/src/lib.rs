//! Human Input Device (HID) Service Implementation.
//!
//! This crate provides access to the Nintendo Switch's HID service, which handles:
//! - Controller input (Npad)
//! - Touch screen
//! - Keyboard
//! - Mouse
//! - Six-axis sensors (gyroscope/accelerometer)
//! - Vibration/rumble
//! - Gesture recognition
//!
//! The HID service uses shared memory (0x40000 bytes) with lock-free LIFO ring
//! buffers for reading input state.

#![no_std]

extern crate nx_panic_handler; // Provide #![panic_handler]

use core::ptr::NonNull;

use nx_service_sm::SmService;
use nx_sf::service::Service;
use nx_svc::ipc::Handle as SessionHandle;
use nx_sys_mem::shmem::{self as sys_shmem, Mapped, Permissions};

mod cmif;
mod proto;
pub mod shmem;

use self::shmem::HidSharedMemory;
pub use self::{
    cmif::{
        ActivateGestureError, ActivateKeyboardError, ActivateMouseError, ActivateNpadError,
        ActivateTouchScreenError, CreateAppletResourceError, GetSharedMemoryHandleError,
        SetSupportedNpadIdTypeError, SetSupportedNpadStyleSetError,
    },
    proto::SERVICE_NAME,
};

/// HID service (IHidServer) session wrapper.
///
/// Provides type safety to distinguish HID sessions from other services.
pub struct HidService {
    service: Service,
    applet_resource: Service,
    shmem_ptr: NonNull<HidSharedMemory>,
    _shmem: sys_shmem::SharedMemory<Mapped>,
    aruid: u64,
}

// SAFETY: HidService is safe to send across threads because:
// - service and applet_resource are just session handles (u32)
// - shmem_ptr points to read-only shared memory that is thread-safe
// - _shmem manages a kernel shared memory handle which is thread-safe
unsafe impl Send for HidService {}

// SAFETY: HidService is safe to share across threads because:
// - All operations are thread-safe
// - Shared memory is read-only and designed for concurrent access
unsafe impl Sync for HidService {}

impl HidService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.service.session
    }

    /// Returns the IAppletResource session handle.
    #[inline]
    pub fn applet_resource_session(&self) -> SessionHandle {
        self.applet_resource.session
    }

    /// Get a reference to the shared memory structure.
    #[inline]
    pub fn shared_memory(&self) -> &HidSharedMemory {
        unsafe { self.shmem_ptr.as_ref() }
    }

    /// Consumes and closes the HID service session.
    #[inline]
    pub fn close(self) {
        self.service.close();
        self.applet_resource.close();
    }

    /// Activate Npad (controller) input.
    #[inline]
    pub fn activate_npad(&self) -> Result<(), ActivateNpadError> {
        cmif::activate_npad(self.service.session, self.aruid)
    }

    /// Set supported Npad style set.
    #[inline]
    pub fn set_supported_npad_style_set(
        &self,
        style_set: u32,
    ) -> Result<(), SetSupportedNpadStyleSetError> {
        cmif::set_supported_npad_style_set(self.service.session, self.aruid, style_set)
    }

    /// Set supported Npad ID types.
    #[inline]
    pub fn set_supported_npad_id_type(
        &self,
        ids: &[u32],
    ) -> Result<(), SetSupportedNpadIdTypeError> {
        cmif::set_supported_npad_id_type(self.service.session, self.aruid, ids)
    }

    /// Activate touch screen input.
    #[inline]
    pub fn activate_touch_screen(&self) -> Result<(), ActivateTouchScreenError> {
        cmif::activate_touch_screen(self.service.session, self.aruid)
    }

    /// Activate keyboard input.
    #[inline]
    pub fn activate_keyboard(&self) -> Result<(), ActivateKeyboardError> {
        cmif::activate_keyboard(self.service.session, self.aruid)
    }

    /// Activate mouse input.
    #[inline]
    pub fn activate_mouse(&self) -> Result<(), ActivateMouseError> {
        cmif::activate_mouse(self.service.session, self.aruid)
    }

    /// Activate gesture recognition.
    #[inline]
    pub fn activate_gesture(&self) -> Result<(), ActivateGestureError> {
        cmif::activate_gesture(self.service.session, self.aruid)
    }
}

/// Connects to the HID service.
///
/// # Arguments
///
/// * `sm` - Service manager session
/// * `aruid` - Applet resource user ID (from applet service)
///
/// # Returns
///
/// A connected [`HidService`] instance on success.
pub fn connect(sm: &SmService, aruid: u64) -> Result<HidService, ConnectError> {
    // Get HID service from service manager
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectError::GetService)?;

    let service = Service {
        session: handle,
        own_handle: 1,
        object_id: 0,
        pointer_buffer_size: 0,
    };

    // Create IAppletResource sub-interface
    let applet_resource_handle = cmif::create_applet_resource(service.session, aruid)
        .map_err(ConnectError::CreateAppletResource)?;

    let applet_resource = Service {
        session: applet_resource_handle,
        own_handle: 1,
        object_id: 0,
        pointer_buffer_size: 0,
    };

    // Get shared memory handle from IAppletResource
    let shmem_handle = cmif::get_shared_memory_handle(applet_resource.session)
        .map_err(ConnectError::GetSharedMemoryHandle)?;

    // Map shared memory (0x40000 bytes, read-only)
    let shmem_unmapped =
        sys_shmem::load_remote(shmem_handle, HidSharedMemory::SIZE, Permissions::R);

    let shmem = unsafe { sys_shmem::map(shmem_unmapped).map_err(ConnectError::MapSharedMemory)? };

    let shmem_ptr = NonNull::new(shmem.addr().unwrap() as *mut HidSharedMemory)
        .ok_or(ConnectError::NullPointer)?;

    Ok(HidService {
        service,
        applet_resource,
        shmem_ptr,
        _shmem: shmem,
        aruid,
    })
}

/// Error returned by [`connect`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    /// Failed to get service handle from SM.
    #[error("failed to get service")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
    /// Failed to create applet resource.
    #[error("failed to create applet resource")]
    CreateAppletResource(#[source] CreateAppletResourceError),
    /// Failed to get shared memory handle.
    #[error("failed to get shared memory handle")]
    GetSharedMemoryHandle(#[source] GetSharedMemoryHandleError),
    /// Failed to map shared memory.
    #[error("failed to map shared memory")]
    MapSharedMemory(#[source] sys_shmem::MapError),
    /// Null pointer from mapped memory.
    #[error("null pointer from mapped memory")]
    NullPointer,
}
