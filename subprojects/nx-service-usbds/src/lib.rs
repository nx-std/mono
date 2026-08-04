//! USB device stack (`usb:ds`) service implementation.
//!
//! Provides the Switch-as-device USB interface for communicating with a host.
//!
//! ## Architecture
//!
//! The service is domain-mode. On 11.0.0+ the root session creates a separate
//! IDsService domain object via cmd 0; on pre-11.0.0 the root session is used
//! directly. Three object levels exist:
//!
//! - [`UsbDsService`] — root service (IDsService)
//! - [`UsbDsInterface`] — interface sub-object (IDsInterface), obtained via
//!   [`UsbDsService::register_interface`] (5.0.0+) or
//!   [`UsbDsService::get_ds_interface_legacy`] (pre-5.0.0)
//! - [`UsbDsEndpoint`] — endpoint sub-object (IDsEndpoint), obtained via
//!   [`UsbDsInterface::register_endpoint`] (5.0.0+) or
//!   [`UsbDsInterface::get_ds_endpoint_legacy`] (pre-5.0.0)
//!
//! ## Divergence from libnx
//!
//! libnx manages global interface/endpoint tables, hosversion-dependent
//! initialization sequences, auto-cleanup, and event caching. This crate
//! exposes each IPC command directly per IC-4, letting callers compose the
//! initialization sequence for their target firmware.
//!
//! The convenience helpers `usbDsWaitReady` and `usbDsParseReportData` are
//! not replicated — callers use the raw commands and types directly.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::{
    ipc::Handle as RawSessionHandle,
    service::{
        BorrowedSessionHandle,
        DispatchError,
        OwnedSessionHandle,
        Session,
    },
};

mod cmif;
mod dispatch;
mod proto;
mod types;

pub use self::{
    cmif::{
        GetEventError,
        GetInterfaceError,
        RegisterEndpointError,
        RegisterInterfaceError,
    },
    proto::SERVICE_NAME,
    types::{
        UsbComplexId,
        UsbDeviceSpeed,
        UsbDsDeviceInfo,
        UsbDsReportData,
        UsbDsReportEntry,
        UsbState,
        UsbStringDescriptor,
    },
};

/// USB device stack (`usb:ds`) root session wrapper (IDsService).
#[repr(transparent)]
pub struct UsbDsService(Session);

impl UsbDsService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// Pre-11.0.0 CMIF protocol methods.
impl UsbDsService {
    /// BindDevice (pre-11.0.0, cmd 0).
    #[inline]
    pub fn bind_device_legacy(&self, complex_id: u32) -> Result<(), DispatchError> {
        cmif::bind_device_legacy(&self.0, complex_id)
    }

    /// SetProcessHandle (pre-11.0.0, cmd 1).
    #[inline]
    pub fn set_process_handle_legacy(&self, proc_handle: u32) -> Result<(), DispatchError> {
        cmif::set_process_handle_legacy(&self.0, proc_handle)
    }

    /// GetDsInterface (pre-5.0.0, cmd 2).
    ///
    /// `descriptor` is the raw `usb_interface_descriptor` bytes (9 bytes).
    /// `interface_name` is the null-terminated interface name.
    ///
    /// Returns a [`UsbDsInterface`] wrapping the domain object and the
    /// assigned interface number.
    pub fn get_ds_interface_legacy(
        &self,
        descriptor: &[u8],
        interface_name: &[u8],
    ) -> Result<(UsbDsInterface, u8), GetInterfaceError> {
        let (raw_handle, intf_num) =
            cmif::get_ds_interface_legacy(&self.0, descriptor, interface_name)?;

        // SAFETY: the kernel returned a valid move handle for the domain object.
        let handle = OwnedSessionHandle::from_handle_unchecked(
            RawSessionHandle::from_raw_unchecked(raw_handle),
        );
        Ok((UsbDsInterface(Session::new(handle, 0)), intf_num))
    }

    /// GetStateChangeEvent (pre-11.0.0, cmd 3). Returns copy-handle.
    #[inline]
    pub fn get_state_change_event_legacy(&self) -> Result<u32, GetEventError> {
        cmif::get_state_change_event(&self.0, proto::GET_STATE_CHANGE_EVENT_LEGACY)
    }

    /// GetState (pre-11.0.0, cmd 4). Returns state as raw u32.
    #[inline]
    pub fn get_state_legacy(&self) -> Result<u32, DispatchError> {
        cmif::get_state(&self.0, proto::GET_STATE_LEGACY)
    }

    /// SetVidPidBcd (pre-5.0.0, cmd 5).
    ///
    /// `deviceinfo` is the raw `UsbDsDeviceInfo` bytes (0x66 bytes).
    #[inline]
    pub fn set_vid_pid_bcd(&self, deviceinfo: &[u8]) -> Result<(), DispatchError> {
        cmif::set_vid_pid_bcd(&self.0, deviceinfo)
    }
}

/// 5.0.0+ CMIF protocol methods (pre-11.0.0 command IDs).
impl UsbDsService {
    /// RegisterInterface (5.0.0–10.x, cmd 2).
    pub fn register_interface_legacy(
        &self,
        intf_num: u8,
    ) -> Result<UsbDsInterface, RegisterInterfaceError> {
        let raw_handle =
            cmif::register_interface(&self.0, proto::GET_DS_INTERFACE_LEGACY, intf_num)?;

        // SAFETY: the kernel returned a valid move handle for the domain object.
        let handle = OwnedSessionHandle::from_handle_unchecked(
            RawSessionHandle::from_raw_unchecked(raw_handle),
        );
        Ok(UsbDsInterface(Session::new(handle, 0)))
    }

    /// ClearDeviceData (5.0.0–10.x, cmd 5).
    #[inline]
    pub fn clear_device_data_legacy(&self) -> Result<(), DispatchError> {
        cmif::clear_device_data(&self.0, proto::CLEAR_DEVICE_DATA_LEGACY)
    }

    /// AddUsbStringDescriptor (5.0.0–10.x, cmd 6). Returns assigned index.
    #[inline]
    pub fn add_usb_string_descriptor_legacy(
        &self,
        descriptor: &UsbStringDescriptor,
    ) -> Result<u8, DispatchError> {
        cmif::add_usb_string_descriptor(
            &self.0,
            proto::ADD_USB_STRING_DESCRIPTOR_LEGACY,
            descriptor,
        )
    }

    /// DeleteUsbStringDescriptor (5.0.0–10.x, cmd 7).
    #[inline]
    pub fn delete_usb_string_descriptor_legacy(&self, index: u8) -> Result<(), DispatchError> {
        cmif::delete_usb_string_descriptor(
            &self.0,
            proto::DELETE_USB_STRING_DESCRIPTOR_LEGACY,
            index,
        )
    }

    /// SetUsbDeviceDescriptor (5.0.0–10.x, cmd 8).
    #[inline]
    pub fn set_usb_device_descriptor_legacy(
        &self,
        speed: u32,
        descriptor: &[u8],
    ) -> Result<(), DispatchError> {
        cmif::set_usb_device_descriptor(
            &self.0,
            proto::SET_USB_DEVICE_DESCRIPTOR_LEGACY,
            speed,
            descriptor,
        )
    }

    /// SetBinaryObjectStore (5.0.0–10.x, cmd 9).
    #[inline]
    pub fn set_binary_object_store_legacy(&self, bos: &[u8]) -> Result<(), DispatchError> {
        cmif::set_binary_object_store(&self.0, proto::SET_BINARY_OBJECT_STORE_LEGACY, bos)
    }

    /// Enable (5.0.0–10.x, cmd 10).
    #[inline]
    pub fn enable_legacy(&self) -> Result<(), DispatchError> {
        cmif::enable(&self.0, proto::ENABLE_LEGACY)
    }

    /// Disable (5.0.0–10.x, cmd 11).
    #[inline]
    pub fn disable_legacy(&self) -> Result<(), DispatchError> {
        cmif::disable(&self.0, proto::DISABLE_LEGACY)
    }

    /// GetSpeed (8.0.0–10.x, cmd 12). Returns speed as raw u32.
    #[inline]
    pub fn get_speed_legacy(&self) -> Result<u32, DispatchError> {
        cmif::get_speed(&self.0, proto::GET_SPEED_LEGACY)
    }
}

/// 11.0.0+ CMIF protocol methods.
impl UsbDsService {
    /// BindDevice (11.0.0+, cmd 0). Sends process handle inline.
    #[inline]
    pub fn bind_device(&self, complex_id: u32, proc_handle: u32) -> Result<(), DispatchError> {
        cmif::bind_device(&self.0, complex_id, proc_handle)
    }

    /// RegisterInterface (11.0.0+, cmd 1).
    pub fn register_interface(
        &self,
        intf_num: u8,
    ) -> Result<UsbDsInterface, RegisterInterfaceError> {
        let raw_handle = cmif::register_interface(&self.0, proto::REGISTER_INTERFACE, intf_num)?;

        // SAFETY: the kernel returned a valid move handle for the domain object.
        let handle = OwnedSessionHandle::from_handle_unchecked(
            RawSessionHandle::from_raw_unchecked(raw_handle),
        );
        Ok(UsbDsInterface(Session::new(handle, 0)))
    }

    /// GetStateChangeEvent (11.0.0+, cmd 2). Returns copy-handle.
    #[inline]
    pub fn get_state_change_event(&self) -> Result<u32, GetEventError> {
        cmif::get_state_change_event(&self.0, proto::GET_STATE_CHANGE_EVENT)
    }

    /// GetState (11.0.0+, cmd 3). Returns state as raw u32.
    #[inline]
    pub fn get_state(&self) -> Result<u32, DispatchError> {
        cmif::get_state(&self.0, proto::GET_STATE)
    }

    /// ClearDeviceData (11.0.0+, cmd 4).
    #[inline]
    pub fn clear_device_data(&self) -> Result<(), DispatchError> {
        cmif::clear_device_data(&self.0, proto::CLEAR_DEVICE_DATA)
    }

    /// AddUsbStringDescriptor (11.0.0+, cmd 5). Returns assigned index.
    #[inline]
    pub fn add_usb_string_descriptor(
        &self,
        descriptor: &UsbStringDescriptor,
    ) -> Result<u8, DispatchError> {
        cmif::add_usb_string_descriptor(&self.0, proto::ADD_USB_STRING_DESCRIPTOR, descriptor)
    }

    /// DeleteUsbStringDescriptor (11.0.0+, cmd 6).
    #[inline]
    pub fn delete_usb_string_descriptor(&self, index: u8) -> Result<(), DispatchError> {
        cmif::delete_usb_string_descriptor(&self.0, proto::DELETE_USB_STRING_DESCRIPTOR, index)
    }

    /// SetUsbDeviceDescriptor (11.0.0+, cmd 7).
    #[inline]
    pub fn set_usb_device_descriptor(
        &self,
        speed: u32,
        descriptor: &[u8],
    ) -> Result<(), DispatchError> {
        cmif::set_usb_device_descriptor(
            &self.0,
            proto::SET_USB_DEVICE_DESCRIPTOR,
            speed,
            descriptor,
        )
    }

    /// SetBinaryObjectStore (11.0.0+, cmd 8).
    #[inline]
    pub fn set_binary_object_store(&self, bos: &[u8]) -> Result<(), DispatchError> {
        cmif::set_binary_object_store(&self.0, proto::SET_BINARY_OBJECT_STORE, bos)
    }

    /// Enable (11.0.0+, cmd 9).
    #[inline]
    pub fn enable(&self) -> Result<(), DispatchError> {
        cmif::enable(&self.0, proto::ENABLE)
    }

    /// Disable (11.0.0+, cmd 10).
    #[inline]
    pub fn disable(&self) -> Result<(), DispatchError> {
        cmif::disable(&self.0, proto::DISABLE)
    }

    /// GetSpeed (11.0.0+, cmd 11). Returns speed as raw u32.
    #[inline]
    pub fn get_speed(&self) -> Result<u32, DispatchError> {
        cmif::get_speed(&self.0, proto::GET_SPEED)
    }
}

/// USB device interface sub-object (IDsInterface).
///
/// Obtained via [`UsbDsService::register_interface`] or
/// [`UsbDsService::get_ds_interface_legacy`]. Owns its own session handle.
#[repr(transparent)]
pub struct UsbDsInterface(Session);

impl UsbDsInterface {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// IDsInterface commands.
impl UsbDsInterface {
    /// GetSetupEvent (cmd 1). Returns copy-handle for the setup event.
    #[inline]
    pub fn get_setup_event(&self) -> Result<u32, GetEventError> {
        cmif::intf_get_event(&self.0, proto::INTF_GET_SETUP_EVENT)
    }

    /// GetSetupPacket (cmd 2). Reads setup packet into buffer.
    #[inline]
    pub fn get_setup_packet(&self, buffer: &mut [u8]) -> Result<(), DispatchError> {
        cmif::intf_get_setup_packet(&self.0, buffer)
    }

    /// EnableInterface (pre-11.0.0, cmd 3). No-op on 11.0.0+.
    #[inline]
    pub fn enable_interface_legacy(&self) -> Result<(), DispatchError> {
        cmif::intf_enable_interface(&self.0)
    }

    /// DisableInterface (pre-11.0.0, cmd 4). No-op on 11.0.0+.
    #[inline]
    pub fn disable_interface_legacy(&self) -> Result<(), DispatchError> {
        cmif::intf_disable_interface(&self.0)
    }

    /// CtrlInPostBufferAsync (pre-11.0.0, cmd 5). Returns urb ID.
    #[inline]
    pub fn ctrl_in_post_buffer_async_legacy(
        &self,
        buffer_addr: u64,
        size: u32,
    ) -> Result<u32, DispatchError> {
        cmif::post_buffer_async(
            &self.0,
            proto::INTF_CTRL_IN_POST_BUFFER_LEGACY,
            buffer_addr,
            size,
        )
    }

    /// CtrlOutPostBufferAsync (pre-11.0.0, cmd 6). Returns urb ID.
    #[inline]
    pub fn ctrl_out_post_buffer_async_legacy(
        &self,
        buffer_addr: u64,
        size: u32,
    ) -> Result<u32, DispatchError> {
        cmif::post_buffer_async(
            &self.0,
            proto::INTF_CTRL_OUT_POST_BUFFER_LEGACY,
            buffer_addr,
            size,
        )
    }

    /// GetCtrlInCompletionEvent (pre-11.0.0, cmd 7). Returns copy-handle.
    #[inline]
    pub fn get_ctrl_in_completion_event_legacy(&self) -> Result<u32, GetEventError> {
        cmif::intf_get_event(&self.0, proto::INTF_GET_CTRL_IN_COMPLETION_EVENT_LEGACY)
    }

    /// GetCtrlInReportData (pre-11.0.0, cmd 8).
    #[inline]
    pub fn get_ctrl_in_report_data_legacy(&self) -> Result<UsbDsReportData, DispatchError> {
        cmif::get_report_data(&self.0, proto::INTF_GET_CTRL_IN_REPORT_DATA_LEGACY)
    }

    /// GetCtrlOutCompletionEvent (pre-11.0.0, cmd 9). Returns copy-handle.
    #[inline]
    pub fn get_ctrl_out_completion_event_legacy(&self) -> Result<u32, GetEventError> {
        cmif::intf_get_event(&self.0, proto::INTF_GET_CTRL_OUT_COMPLETION_EVENT_LEGACY)
    }

    /// GetCtrlOutReportData (pre-11.0.0, cmd 10).
    #[inline]
    pub fn get_ctrl_out_report_data_legacy(&self) -> Result<UsbDsReportData, DispatchError> {
        cmif::get_report_data(&self.0, proto::INTF_GET_CTRL_OUT_REPORT_DATA_LEGACY)
    }

    /// StallCtrl (pre-11.0.0, cmd 11).
    #[inline]
    pub fn stall_ctrl_legacy(&self) -> Result<(), DispatchError> {
        cmif::stall_ctrl(&self.0, proto::INTF_STALL_CTRL_LEGACY)
    }

    /// CtrlInPostBufferAsync (11.0.0+, cmd 3). Returns urb ID.
    #[inline]
    pub fn ctrl_in_post_buffer_async(
        &self,
        buffer_addr: u64,
        size: u32,
    ) -> Result<u32, DispatchError> {
        cmif::post_buffer_async(&self.0, proto::INTF_CTRL_IN_POST_BUFFER, buffer_addr, size)
    }

    /// CtrlOutPostBufferAsync (11.0.0+, cmd 4). Returns urb ID.
    #[inline]
    pub fn ctrl_out_post_buffer_async(
        &self,
        buffer_addr: u64,
        size: u32,
    ) -> Result<u32, DispatchError> {
        cmif::post_buffer_async(&self.0, proto::INTF_CTRL_OUT_POST_BUFFER, buffer_addr, size)
    }

    /// GetCtrlInCompletionEvent (11.0.0+, cmd 5). Returns copy-handle.
    #[inline]
    pub fn get_ctrl_in_completion_event(&self) -> Result<u32, GetEventError> {
        cmif::intf_get_event(&self.0, proto::INTF_GET_CTRL_IN_COMPLETION_EVENT)
    }

    /// GetCtrlInReportData (11.0.0+, cmd 6).
    #[inline]
    pub fn get_ctrl_in_report_data(&self) -> Result<UsbDsReportData, DispatchError> {
        cmif::get_report_data(&self.0, proto::INTF_GET_CTRL_IN_REPORT_DATA)
    }

    /// GetCtrlOutCompletionEvent (11.0.0+, cmd 7). Returns copy-handle.
    #[inline]
    pub fn get_ctrl_out_completion_event(&self) -> Result<u32, GetEventError> {
        cmif::intf_get_event(&self.0, proto::INTF_GET_CTRL_OUT_COMPLETION_EVENT)
    }

    /// GetCtrlOutReportData (11.0.0+, cmd 8).
    #[inline]
    pub fn get_ctrl_out_report_data(&self) -> Result<UsbDsReportData, DispatchError> {
        cmif::get_report_data(&self.0, proto::INTF_GET_CTRL_OUT_REPORT_DATA)
    }

    /// StallCtrl (11.0.0+, cmd 9).
    #[inline]
    pub fn stall_ctrl(&self) -> Result<(), DispatchError> {
        cmif::stall_ctrl(&self.0, proto::INTF_STALL_CTRL)
    }

    /// GetDsEndpoint (pre-5.0.0, cmd 0). Takes raw endpoint descriptor bytes.
    pub fn get_ds_endpoint_legacy(
        &self,
        descriptor: &[u8],
    ) -> Result<UsbDsEndpoint, RegisterEndpointError> {
        let raw_handle = cmif::intf_get_ds_endpoint(&self.0, descriptor)?;

        // SAFETY: the kernel returned a valid move handle for the domain object.
        let handle = OwnedSessionHandle::from_handle_unchecked(
            RawSessionHandle::from_raw_unchecked(raw_handle),
        );
        Ok(UsbDsEndpoint(Session::new(handle, 0)))
    }

    /// RegisterEndpoint (5.0.0+, cmd 0). Takes endpoint address.
    pub fn register_endpoint(
        &self,
        endpoint_address: u8,
    ) -> Result<UsbDsEndpoint, RegisterEndpointError> {
        let raw_handle = cmif::intf_register_endpoint(&self.0, endpoint_address)?;

        // SAFETY: the kernel returned a valid move handle for the domain object.
        let handle = OwnedSessionHandle::from_handle_unchecked(
            RawSessionHandle::from_raw_unchecked(raw_handle),
        );
        Ok(UsbDsEndpoint(Session::new(handle, 0)))
    }

    /// AppendConfigurationData (pre-11.0.0, cmd 12).
    ///
    /// `intf_num` is the interface number assigned during registration.
    #[inline]
    pub fn append_configuration_data_legacy(
        &self,
        intf_num: u8,
        speed: u32,
        buffer: &[u8],
    ) -> Result<(), DispatchError> {
        cmif::intf_append_configuration_data_legacy(&self.0, intf_num, speed, buffer)
    }

    /// AppendConfigurationData (11.0.0+, cmd 10).
    #[inline]
    pub fn append_configuration_data(
        &self,
        speed: u32,
        buffer: &[u8],
    ) -> Result<(), DispatchError> {
        cmif::intf_append_configuration_data(&self.0, speed, buffer)
    }
}

/// USB device endpoint sub-object (IDsEndpoint).
///
/// Obtained via [`UsbDsInterface::register_endpoint`] or
/// [`UsbDsInterface::get_ds_endpoint_legacy`]. Owns its own session handle.
#[repr(transparent)]
pub struct UsbDsEndpoint(Session);

impl UsbDsEndpoint {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// IDsEndpoint commands.
impl UsbDsEndpoint {
    /// PostBufferAsync (cmd 0). Returns urb ID.
    #[inline]
    pub fn post_buffer_async(&self, buffer_addr: u64, size: u32) -> Result<u32, DispatchError> {
        cmif::post_buffer_async(&self.0, proto::EP_POST_BUFFER_ASYNC, buffer_addr, size)
    }

    /// Cancel (cmd 1).
    #[inline]
    pub fn cancel(&self) -> Result<(), DispatchError> {
        cmif::ep_cancel(&self.0)
    }

    /// GetCompletionEvent (cmd 2). Returns copy-handle.
    #[inline]
    pub fn get_completion_event(&self) -> Result<u32, GetEventError> {
        cmif::intf_get_event(&self.0, proto::EP_GET_COMPLETION_EVENT)
    }

    /// GetReportData (cmd 3).
    #[inline]
    pub fn get_report_data(&self) -> Result<UsbDsReportData, DispatchError> {
        cmif::get_report_data(&self.0, proto::EP_GET_REPORT_DATA)
    }

    /// Stall (cmd 4).
    #[inline]
    pub fn stall(&self) -> Result<(), DispatchError> {
        cmif::ep_stall(&self.0)
    }

    /// SetZlt — sets zero-length termination (cmd 5).
    #[inline]
    pub fn set_zlt(&self, zlt: bool) -> Result<(), DispatchError> {
        cmif::ep_set_zlt(&self.0, zlt)
    }
}

/// Connects to the `usb:ds` service using CMIF.
///
/// The caller is responsible for converting to domain mode and performing
/// the initialization sequence (BindDevice, GetStateChangeEvent, etc.)
/// appropriate for their target firmware version.
pub fn connect_cmif(sm: &SmService) -> Result<UsbDsService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::new(handle, 0);

    Ok(UsbDsService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get usb:ds service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
