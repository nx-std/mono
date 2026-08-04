//! HID Bus service (`hidbus`) implementation.
//!
//! Provides access to external devices attached to HID controllers via the
//! hidbus service. Only available on \[5.0.0+\].
//!
//! ## Usage
//!
//! 1. Connect to the service via [`connect_cmif`].
//! 2. Use [`HidbusService::get_bus_handle`] to obtain a [`BusHandle`].
//! 3. Initialize the bus handle, then enable the external device.
//! 4. Use async send/receive for device communication.
//! 5. Shared memory provides device status via [`StatusManager`] / [`StatusManagerV5`].

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::{
    BorrowedSessionHandle,
    Session,
};

mod cmif;
mod proto;
mod types;

pub use self::{
    cmif::{
        DispatchError,
        EnableJoyPollingError,
        GetSendCommandAsyncResultError,
        GetSharedMemoryError,
        SetEventError,
    },
    proto::SERVICE_NAME,
    types::{
        BusHandle,
        BusType,
        DataAccessorHeader,
        JoyButtonOnlyPollingDataAccessor,
        JoyButtonOnlyPollingEntry,
        JoyButtonOnlyPollingEntryData,
        JoyDisableSixAxisPollingDataAccessor,
        JoyDisableSixAxisPollingEntry,
        JoyDisableSixAxisPollingEntryData,
        JoyEnableSixAxisPollingDataAccessor,
        JoyEnableSixAxisPollingEntry,
        JoyEnableSixAxisPollingEntryData,
        JoyPollingMode,
        JoyPollingReceivedData,
        StatusManager,
        StatusManagerEntry,
        StatusManagerEntryCommon,
        StatusManagerEntryV5,
        StatusManagerV5,
    },
};

/// HID Bus service wrapper.
#[repr(transparent)]
pub struct HidbusService(Session);

impl HidbusService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }

    /// Gets a bus handle for the given controller and bus type.
    ///
    /// Returns `(is_valid, handle)`.
    #[inline]
    pub fn get_bus_handle(
        &self,
        npad_id: u32,
        bus_type: BusType,
        applet_resource_user_id: u64,
    ) -> Result<(bool, BusHandle), DispatchError> {
        cmif::get_bus_handle(
            self.0.handle(),
            npad_id,
            bus_type as u64,
            applet_resource_user_id,
        )
    }

    /// Initializes a bus handle for use.
    #[inline]
    pub fn initialize(
        &self,
        handle: BusHandle,
        applet_resource_user_id: u64,
    ) -> Result<(), DispatchError> {
        cmif::initialize(self.0.handle(), handle, applet_resource_user_id)
    }

    /// Finalizes a bus handle.
    #[inline]
    pub fn finalize(
        &self,
        handle: BusHandle,
        applet_resource_user_id: u64,
    ) -> Result<(), DispatchError> {
        cmif::finalize(self.0.handle(), handle, applet_resource_user_id)
    }

    /// Enables or disables the external device on the given bus handle.
    #[inline]
    pub fn enable_external_device(
        &self,
        handle: BusHandle,
        flag: bool,
        inval: u64,
        applet_resource_user_id: u64,
    ) -> Result<(), DispatchError> {
        cmif::enable_external_device(
            self.0.handle(),
            handle,
            flag,
            inval,
            applet_resource_user_id,
        )
    }

    /// Gets the external device ID for the given bus handle.
    #[inline]
    pub fn get_external_device_id(&self, handle: BusHandle) -> Result<u32, DispatchError> {
        cmif::get_external_device_id(self.0.handle(), handle)
    }

    /// Sends a command asynchronously to the external device.
    #[inline]
    pub fn send_command_async(
        &self,
        handle: BusHandle,
        buffer: &[u8],
    ) -> Result<(), DispatchError> {
        cmif::send_command_async(self.0.handle(), handle, buffer)
    }

    /// Gets the result of an async send command, writing reply data into `buffer`.
    ///
    /// Returns the actual output size.
    #[inline]
    pub fn get_send_command_async_result(
        &self,
        handle: BusHandle,
        buffer: &mut [u8],
    ) -> Result<u32, GetSendCommandAsyncResultError> {
        cmif::get_send_command_async_result(self.0.handle(), handle, buffer)
    }

    /// Gets a copy handle for the async send command completion event.
    #[inline]
    pub fn set_event_for_send_command_async_result(
        &self,
        handle: BusHandle,
    ) -> Result<u32, SetEventError> {
        cmif::set_event_for_send_command_async_result(self.0.handle(), handle)
    }

    /// Gets a copy handle for the shared memory containing device status.
    #[inline]
    pub fn get_shared_memory_handle(&self) -> Result<u32, GetSharedMemoryError> {
        cmif::get_shared_memory_handle(self.0.handle())
    }

    /// Enables joy polling receive mode with a transfer memory buffer.
    ///
    /// `tmem_handle` is the transfer memory handle. `tmem_size` is the
    /// transfer memory size. `command_buffer` contains the polling command data.
    #[inline]
    pub fn enable_joy_polling_receive_mode(
        &self,
        handle: BusHandle,
        polling_mode: JoyPollingMode,
        command_buffer: &[u8],
        tmem_handle: u32,
        tmem_size: u32,
    ) -> Result<(), EnableJoyPollingError> {
        cmif::enable_joy_polling_receive_mode(
            self.0.handle(),
            handle,
            polling_mode,
            command_buffer,
            tmem_handle,
            tmem_size,
        )
    }

    /// Disables joy polling receive mode.
    #[inline]
    pub fn disable_joy_polling_receive_mode(&self, handle: BusHandle) -> Result<(), DispatchError> {
        cmif::disable_joy_polling_receive_mode(self.0.handle(), handle)
    }

    /// Sets the status manager type.
    #[inline]
    pub fn set_status_manager_type(&self, manager_type: u32) -> Result<(), DispatchError> {
        cmif::set_status_manager_type(self.0.handle(), manager_type)
    }
}

/// Connects to the HID Bus service (`hidbus`) using CMIF.
pub fn connect_cmif(sm: &SmService) -> Result<HidbusService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::new(handle, 0);

    Ok(HidbusService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get hidbus service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
