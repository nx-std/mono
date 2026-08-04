//! USB host stack (`usb:hs`) service implementation.
//!
//! Provides the Switch USB host interface for communicating with USB devices.
//!
//! ## Architecture
//!
//! The service is domain-mode. Three object levels exist:
//!
//! - [`UsbHsService`] — root service (IUsbHsService)
//! - [`UsbHsClientIf`] — interface session (IClientIfSession), obtained via
//!   [`UsbHsService::acquire_usb_if`] or [`UsbHsService::acquire_usb_if_legacy`]
//! - [`UsbHsClientEp`] — endpoint session (IClientEpSession), obtained via
//!   [`UsbHsClientIf::open_usb_ep`] or [`UsbHsClientIf::open_usb_ep_legacy`]
//!
//! ## Divergence from libnx
//!
//! libnx manages global state, hosversion-dependent initialization, endpoint
//! descriptor byte-swapping (pre-8.0.0 vs 8.0.0+), and convenience wrappers
//! that combine async+event+report. This crate exposes each IPC command
//! directly per IC-4, letting callers compose the initialization sequence for
//! their target firmware.
//!
//! The pre-8.0.0 INPUT/OUTPUT endpoint descriptor swap
//! (`_usbHsConvertInterfaceInfoToV8`) is not performed — callers targeting
//! pre-8.0.0 must swap the arrays themselves.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::{
    ipc::Handle as RawSessionHandle,
    service::{
        BorrowedSessionHandle,
        BufferAttr,
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
        AcquireIfError,
        GetEventError,
        OpenEpError,
    },
    proto::SERVICE_NAME,
    types::{
        UsbConfigDescriptor,
        UsbDeviceDescriptor,
        UsbEndpointDescriptor,
        UsbHsInterface,
        UsbHsInterfaceFilter,
        UsbHsInterfaceFilterFlags,
        UsbHsInterfaceInfo,
        UsbHsRingHeader,
        UsbHsXferReport,
        UsbInterfaceDescriptor,
        UsbSsEndpointCompanionDescriptor,
    },
};

// ---------------------------------------------------------------------------
// UsbHsService — root session
// ---------------------------------------------------------------------------

/// USB host stack (`usb:hs`) root session wrapper (IUsbHsService).
#[repr(transparent)]
pub struct UsbHsService(Session);

impl UsbHsService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// 2.0.0+ root service commands.
impl UsbHsService {
    /// BindClientProcess (2.0.0+, cmd 0). Sends process handle as copy-handle.
    #[inline]
    pub fn bind_client_process(&self, proc_handle: u32) -> Result<(), DispatchError> {
        cmif::bind_client_process(&self.0, proc_handle)
    }

    /// QueryAllInterfaces (2.0.0+, cmd 1).
    ///
    /// Returns the number of interfaces written to the output buffer.
    #[inline]
    pub fn query_all_interfaces(
        &self,
        filter: &UsbHsInterfaceFilter,
        interfaces: &mut [UsbHsInterface],
    ) -> Result<i32, DispatchError> {
        cmif::query_interfaces_with_filter(
            &self.0,
            proto::QUERY_ALL_INTERFACES,
            filter,
            interfaces.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(interfaces),
        )
    }

    /// QueryAvailableInterfaces (2.0.0+, cmd 2).
    ///
    /// Returns the number of interfaces written to the output buffer.
    #[inline]
    pub fn query_available_interfaces(
        &self,
        filter: &UsbHsInterfaceFilter,
        interfaces: &mut [UsbHsInterface],
    ) -> Result<i32, DispatchError> {
        cmif::query_interfaces_with_filter(
            &self.0,
            proto::QUERY_AVAILABLE_INTERFACES,
            filter,
            interfaces.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(interfaces),
        )
    }

    /// QueryAcquiredInterfaces (2.0.0+, cmd 3).
    ///
    /// Returns the number of interfaces written to the output buffer.
    #[inline]
    pub fn query_acquired_interfaces(
        &self,
        interfaces: &mut [UsbHsInterface],
    ) -> Result<i32, DispatchError> {
        cmif::query_acquired_interfaces(
            &self.0,
            proto::QUERY_ACQUIRED_INTERFACES,
            interfaces.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(interfaces),
        )
    }

    /// CreateInterfaceAvailableEvent (2.0.0+, cmd 4).
    ///
    /// `index` must be 0..2. Returns a copy-handle for the event.
    #[inline]
    pub fn create_interface_available_event(
        &self,
        index: u8,
        filter: &UsbHsInterfaceFilter,
    ) -> Result<u32, GetEventError> {
        cmif::create_interface_available_event(
            &self.0,
            proto::CREATE_INTERFACE_AVAILABLE_EVENT,
            index,
            filter,
        )
    }

    /// DestroyInterfaceAvailableEvent (2.0.0+, cmd 5).
    #[inline]
    pub fn destroy_interface_available_event(&self, index: u8) -> Result<(), DispatchError> {
        cmif::destroy_interface_available_event(
            &self.0,
            proto::DESTROY_INTERFACE_AVAILABLE_EVENT,
            index,
        )
    }

    /// GetInterfaceStateChangeEvent (2.0.0+, cmd 6). Returns copy-handle.
    #[inline]
    pub fn get_interface_state_change_event(&self) -> Result<u32, GetEventError> {
        cmif::get_event(&self.0, proto::GET_INTERFACE_STATE_CHANGE_EVENT)
    }

    /// AcquireUsbIf (2.0.0+, cmd 7).
    ///
    /// Acquires the interface identified by `interface_id`. Output is written
    /// into the two regions of `intf_out`: the pathstr area and the
    /// `UsbHsInterfaceInfo`.
    ///
    /// Returns a [`UsbHsClientIf`] wrapping the acquired interface session.
    pub fn acquire_usb_if(
        &self,
        interface_id: i32,
        intf_out: &mut UsbHsInterface,
    ) -> Result<UsbHsClientIf, AcquireIfError> {
        let intf_data_size =
            core::mem::size_of::<UsbHsInterface>() - core::mem::size_of::<UsbHsInterfaceInfo>();

        let raw_handle = cmif::acquire_usb_if(
            &self.0,
            interface_id,
            (&raw mut intf_out.pathstr).cast::<u8>(),
            intf_data_size,
            &raw mut intf_out.inf,
        )?;

        // SAFETY: the kernel returned a valid move handle for the domain object.
        let handle = OwnedSessionHandle::from_handle_unchecked(
            RawSessionHandle::from_raw_unchecked(raw_handle),
        );
        Ok(UsbHsClientIf(Session::new(handle, 0)))
    }
}

/// Pre-2.0.0 root service commands.
impl UsbHsService {
    /// QueryAllInterfaces (pre-2.0.0, cmd 0).
    #[inline]
    pub fn query_all_interfaces_legacy(
        &self,
        filter: &UsbHsInterfaceFilter,
        interfaces: &mut [UsbHsInterface],
    ) -> Result<i32, DispatchError> {
        cmif::query_interfaces_with_filter(
            &self.0,
            proto::QUERY_ALL_INTERFACES_LEGACY,
            filter,
            interfaces.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(interfaces),
        )
    }

    /// QueryAvailableInterfaces (pre-2.0.0, cmd 1).
    #[inline]
    pub fn query_available_interfaces_legacy(
        &self,
        filter: &UsbHsInterfaceFilter,
        interfaces: &mut [UsbHsInterface],
    ) -> Result<i32, DispatchError> {
        cmif::query_interfaces_with_filter(
            &self.0,
            proto::QUERY_AVAILABLE_INTERFACES_LEGACY,
            filter,
            interfaces.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(interfaces),
        )
    }

    /// QueryAcquiredInterfaces (pre-2.0.0, cmd 2).
    #[inline]
    pub fn query_acquired_interfaces_legacy(
        &self,
        interfaces: &mut [UsbHsInterface],
    ) -> Result<i32, DispatchError> {
        cmif::query_acquired_interfaces(
            &self.0,
            proto::QUERY_ACQUIRED_INTERFACES_LEGACY,
            interfaces.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(interfaces),
        )
    }

    /// CreateInterfaceAvailableEvent (pre-2.0.0, cmd 3).
    #[inline]
    pub fn create_interface_available_event_legacy(
        &self,
        index: u8,
        filter: &UsbHsInterfaceFilter,
    ) -> Result<u32, GetEventError> {
        cmif::create_interface_available_event(
            &self.0,
            proto::CREATE_INTERFACE_AVAILABLE_EVENT_LEGACY,
            index,
            filter,
        )
    }

    /// DestroyInterfaceAvailableEvent (pre-2.0.0, cmd 4).
    #[inline]
    pub fn destroy_interface_available_event_legacy(&self, index: u8) -> Result<(), DispatchError> {
        cmif::destroy_interface_available_event(
            &self.0,
            proto::DESTROY_INTERFACE_AVAILABLE_EVENT_LEGACY,
            index,
        )
    }

    /// GetInterfaceStateChangeEvent (pre-2.0.0, cmd 5). Returns copy-handle.
    #[inline]
    pub fn get_interface_state_change_event_legacy(&self) -> Result<u32, GetEventError> {
        cmif::get_event(&self.0, proto::GET_INTERFACE_STATE_CHANGE_EVENT_LEGACY)
    }

    /// AcquireUsbIf (pre-2.0.0, cmd 6).
    ///
    /// Acquires the interface identified by `interface_id`. Output
    /// `UsbHsInterfaceInfo` is written to `info_out`.
    ///
    /// Returns a [`UsbHsClientIf`] wrapping the acquired interface session.
    pub fn acquire_usb_if_legacy(
        &self,
        interface_id: i32,
        info_out: &mut UsbHsInterfaceInfo,
    ) -> Result<UsbHsClientIf, AcquireIfError> {
        let raw_handle = cmif::acquire_usb_if_legacy(&self.0, interface_id, &raw mut *info_out)?;

        // SAFETY: the kernel returned a valid move handle for the domain object.
        let handle = OwnedSessionHandle::from_handle_unchecked(
            RawSessionHandle::from_raw_unchecked(raw_handle),
        );
        Ok(UsbHsClientIf(Session::new(handle, 0)))
    }
}

// ---------------------------------------------------------------------------
// UsbHsClientIf — interface session
// ---------------------------------------------------------------------------

/// USB host interface session (IClientIfSession).
///
/// Obtained via [`UsbHsService::acquire_usb_if`] or
/// [`UsbHsService::acquire_usb_if_legacy`]. Owns its own session handle.
#[repr(transparent)]
pub struct UsbHsClientIf(Session);

impl UsbHsClientIf {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// Version-independent interface commands.
impl UsbHsClientIf {
    /// GetCtrlXferEvent (cmd 0, all versions). Returns copy-handle.
    #[inline]
    pub fn get_ctrl_xfer_event(&self) -> Result<u32, GetEventError> {
        cmif::get_event(&self.0, proto::IF_GET_CTRL_XFER_EVENT)
    }

    /// SetInterface (cmd 1). Writes output to `info_out`.
    #[inline]
    pub fn set_interface(
        &self,
        id: u8,
        info_out: &mut UsbHsInterfaceInfo,
    ) -> Result<(), DispatchError> {
        cmif::if_set_interface(&self.0, id, &raw mut *info_out)
    }

    /// GetInterface (cmd 2). Writes output to `info_out`.
    #[inline]
    pub fn get_interface(&self, info_out: &mut UsbHsInterfaceInfo) -> Result<(), DispatchError> {
        cmif::if_get_interface(&self.0, &raw mut *info_out)
    }

    /// GetAlternateInterface (cmd 3). Writes output to `info_out`.
    #[inline]
    pub fn get_alternate_interface(
        &self,
        id: u8,
        info_out: &mut UsbHsInterfaceInfo,
    ) -> Result<(), DispatchError> {
        cmif::if_get_alternate_interface(&self.0, id, &raw mut *info_out)
    }

    /// ResetDevice (cmd 8).
    #[inline]
    pub fn reset_device(&self) -> Result<(), DispatchError> {
        dispatch::dispatch_domain_no_io(&self.0, proto::IF_RESET_DEVICE)
    }
}

/// Pre-2.0.0 interface commands.
impl UsbHsClientIf {
    /// GetCurrentFrame (pre-2.0.0, cmd 5).
    #[inline]
    pub fn get_current_frame_legacy(&self) -> Result<u32, DispatchError> {
        cmif::if_get_current_frame(&self.0, proto::IF_GET_CURRENT_FRAME_LEGACY)
    }

    /// SubmitControlRequest IN (pre-2.0.0, cmd 6).
    /// Performs a blocking control transfer that reads from the device.
    ///
    /// Returns the number of bytes transferred.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn submit_control_request_in(
        &self,
        b_request: u8,
        bm_request_type: u8,
        w_value: u16,
        w_index: u16,
        w_length: u16,
        buffer: &mut [u8],
        timeout_in_ms: u32,
    ) -> Result<u32, DispatchError> {
        cmif::if_submit_control_request(
            &self.0,
            proto::IF_SUBMIT_CONTROL_REQUEST_IN,
            b_request,
            bm_request_type,
            w_value,
            w_index,
            w_length,
            buffer.as_mut_ptr(),
            buffer.len(),
            timeout_in_ms,
        )
    }

    /// SubmitControlRequest OUT (pre-2.0.0, cmd 7).
    /// Performs a blocking control transfer that writes to the device.
    ///
    /// Returns the number of bytes transferred.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn submit_control_request_out(
        &self,
        b_request: u8,
        bm_request_type: u8,
        w_value: u16,
        w_index: u16,
        w_length: u16,
        buffer: &[u8],
        timeout_in_ms: u32,
    ) -> Result<u32, DispatchError> {
        cmif::if_submit_control_request(
            &self.0,
            proto::IF_SUBMIT_CONTROL_REQUEST_OUT,
            b_request,
            bm_request_type,
            w_value,
            w_index,
            w_length,
            buffer.as_ptr().cast_mut(),
            buffer.len(),
            timeout_in_ms,
        )
    }

    /// OpenUsbEp (pre-2.0.0, cmd 4).
    ///
    /// Opens an endpoint. Returns the endpoint session and the resolved
    /// endpoint descriptor.
    pub fn open_usb_ep_legacy(
        &self,
        max_urb_count: u16,
        max_xfer_size: u32,
        ep_type: u32,
        ep_number: u32,
        ep_direction: u32,
    ) -> Result<(UsbHsClientEp, UsbEndpointDescriptor), OpenEpError> {
        let (raw_handle, desc) = cmif::if_open_usb_ep(
            &self.0,
            proto::IF_OPEN_USB_EP_LEGACY,
            max_urb_count,
            max_xfer_size,
            ep_type,
            ep_number,
            ep_direction,
        )?;

        // SAFETY: the kernel returned a valid move handle for the domain object.
        let handle = OwnedSessionHandle::from_handle_unchecked(
            RawSessionHandle::from_raw_unchecked(raw_handle),
        );
        Ok((UsbHsClientEp(Session::new(handle, 0)), desc))
    }
}

/// 2.0.0+ interface commands.
impl UsbHsClientIf {
    /// GetCurrentFrame (2.0.0+, cmd 4).
    #[inline]
    pub fn get_current_frame(&self) -> Result<u32, DispatchError> {
        cmif::if_get_current_frame(&self.0, proto::IF_GET_CURRENT_FRAME)
    }

    /// CtrlXferAsync (2.0.0+, cmd 5).
    ///
    /// Initiates an asynchronous control transfer. Use
    /// [`get_ctrl_xfer_completion_event`](Self::get_ctrl_xfer_completion_event)
    /// and [`get_ctrl_xfer_report`](Self::get_ctrl_xfer_report) to wait for
    /// and retrieve the result.
    #[inline]
    pub fn ctrl_xfer_async(
        &self,
        bm_request_type: u8,
        b_request: u8,
        w_value: u16,
        w_index: u16,
        w_length: u16,
        buffer: u64,
    ) -> Result<(), DispatchError> {
        cmif::if_ctrl_xfer_async(
            &self.0,
            bm_request_type,
            b_request,
            w_value,
            w_index,
            w_length,
            buffer,
        )
    }

    /// GetCtrlXferCompletionEvent (2.0.0+, cmd 6). Returns copy-handle.
    #[inline]
    pub fn get_ctrl_xfer_completion_event(&self) -> Result<u32, GetEventError> {
        cmif::get_event(&self.0, proto::IF_GET_CTRL_XFER_COMPLETION_EVENT)
    }

    /// GetCtrlXferReport (2.0.0+, cmd 7).
    #[inline]
    pub fn get_ctrl_xfer_report(
        &self,
        report_out: &mut UsbHsXferReport,
    ) -> Result<(), DispatchError> {
        cmif::if_get_ctrl_xfer_report(&self.0, &raw mut *report_out)
    }

    /// OpenUsbEp (2.0.0+, cmd 9).
    ///
    /// Opens an endpoint. Returns the endpoint session and the resolved
    /// endpoint descriptor.
    pub fn open_usb_ep(
        &self,
        max_urb_count: u16,
        max_xfer_size: u32,
        ep_type: u32,
        ep_number: u32,
        ep_direction: u32,
    ) -> Result<(UsbHsClientEp, UsbEndpointDescriptor), OpenEpError> {
        let (raw_handle, desc) = cmif::if_open_usb_ep(
            &self.0,
            proto::IF_OPEN_USB_EP,
            max_urb_count,
            max_xfer_size,
            ep_type,
            ep_number,
            ep_direction,
        )?;

        // SAFETY: the kernel returned a valid move handle for the domain object.
        let handle = OwnedSessionHandle::from_handle_unchecked(
            RawSessionHandle::from_raw_unchecked(raw_handle),
        );
        Ok((UsbHsClientEp(Session::new(handle, 0)), desc))
    }
}

// ---------------------------------------------------------------------------
// UsbHsClientEp — endpoint session
// ---------------------------------------------------------------------------

/// USB host endpoint session (IClientEpSession).
///
/// Obtained via [`UsbHsClientIf::open_usb_ep`] or
/// [`UsbHsClientIf::open_usb_ep_legacy`]. Owns its own session handle.
#[repr(transparent)]
pub struct UsbHsClientEp(Session);

impl UsbHsClientEp {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// Pre-2.0.0 endpoint commands.
impl UsbHsClientEp {
    /// SubmitRequest OUT (pre-2.0.0, cmd 0). Blocking send transfer.
    ///
    /// Returns the number of bytes transferred.
    #[inline]
    pub fn submit_request_out(
        &self,
        size: u32,
        timeout_in_ms: u32,
        buffer: &[u8],
    ) -> Result<u32, DispatchError> {
        cmif::ep_submit_request(
            &self.0,
            proto::EP_SUBMIT_REQUEST_OUT,
            size,
            timeout_in_ms,
            buffer.as_ptr().cast_mut(),
            buffer.len(),
        )
    }

    /// SubmitRequest IN (pre-2.0.0, cmd 1). Blocking receive transfer.
    ///
    /// Returns the number of bytes transferred.
    #[inline]
    pub fn submit_request_in(
        &self,
        size: u32,
        timeout_in_ms: u32,
        buffer: &mut [u8],
    ) -> Result<u32, DispatchError> {
        cmif::ep_submit_request(
            &self.0,
            proto::EP_SUBMIT_REQUEST_IN,
            size,
            timeout_in_ms,
            buffer.as_mut_ptr(),
            buffer.len(),
        )
    }

    /// Close (pre-2.0.0, cmd 3). Notifies the server before session teardown.
    #[inline]
    pub fn close_legacy(&self) -> Result<(), DispatchError> {
        dispatch::dispatch_domain_no_io(&self.0, proto::EP_CLOSE_LEGACY)
    }
}

/// 2.0.0+ endpoint commands.
impl UsbHsClientEp {
    /// Close (2.0.0+, cmd 1). Notifies the server before session teardown.
    #[inline]
    pub fn close(&self) -> Result<(), DispatchError> {
        dispatch::dispatch_domain_no_io(&self.0, proto::EP_CLOSE)
    }

    /// GetXferEvent (2.0.0+, cmd 2). Returns copy-handle for the event.
    #[inline]
    pub fn get_xfer_event(&self) -> Result<u32, GetEventError> {
        cmif::get_event(&self.0, proto::EP_GET_XFER_EVENT)
    }

    /// Populate (2.0.0+, cmd 3).
    #[inline]
    pub fn populate(&self) -> Result<(), DispatchError> {
        dispatch::dispatch_domain_no_io(&self.0, proto::EP_POPULATE)
    }

    /// PostBufferAsync (2.0.0+, cmd 4).
    ///
    /// `buffer` is the device-mapped address. `id` is an arbitrary value
    /// returned in the transfer report.
    ///
    /// Returns the transfer ID.
    #[inline]
    pub fn post_buffer_async(&self, size: u32, buffer: u64, id: u64) -> Result<u32, DispatchError> {
        cmif::ep_post_buffer_async(&self.0, size, buffer, id)
    }

    /// GetXferReport (pre-3.0.0, cmd 5). Uses HipcMapAlias buffers.
    ///
    /// Returns the number of reports written to the output slice.
    #[inline]
    pub fn get_xfer_report_legacy(
        &self,
        reports: &mut [UsbHsXferReport],
    ) -> Result<u32, DispatchError> {
        cmif::ep_get_xfer_report(
            &self.0,
            reports.len() as u32,
            reports.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(reports),
            BufferAttr::OUT.or(BufferAttr::HIPC_MAP_ALIAS),
        )
    }

    /// GetXferReport (3.0.0+, cmd 5). Uses HipcAutoSelect buffers.
    ///
    /// Returns the number of reports written to the output slice.
    #[inline]
    pub fn get_xfer_report(&self, reports: &mut [UsbHsXferReport]) -> Result<u32, DispatchError> {
        cmif::ep_get_xfer_report(
            &self.0,
            reports.len() as u32,
            reports.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(reports),
            BufferAttr::OUT.or(BufferAttr::HIPC_AUTO_SELECT),
        )
    }

    /// BatchBufferAsync (pre-3.0.0, cmd 6). Uses HipcMapAlias buffers.
    ///
    /// Returns the transfer ID.
    #[inline]
    pub fn batch_buffer_async_legacy(
        &self,
        buffer: u64,
        urbs: &[u32],
        id: u64,
        unk1: u32,
        unk2: u32,
    ) -> Result<u32, DispatchError> {
        cmif::ep_batch_buffer_async(
            &self.0,
            urbs.len() as u32,
            unk1,
            unk2,
            buffer,
            id,
            urbs.as_ptr().cast::<u8>(),
            core::mem::size_of_val(urbs),
            BufferAttr::IN.or(BufferAttr::HIPC_MAP_ALIAS),
        )
    }

    /// BatchBufferAsync (3.0.0+, cmd 6). Uses HipcAutoSelect buffers.
    ///
    /// Returns the transfer ID.
    #[inline]
    pub fn batch_buffer_async(
        &self,
        buffer: u64,
        urbs: &[u32],
        id: u64,
        unk1: u32,
        unk2: u32,
    ) -> Result<u32, DispatchError> {
        cmif::ep_batch_buffer_async(
            &self.0,
            urbs.len() as u32,
            unk1,
            unk2,
            buffer,
            id,
            urbs.as_ptr().cast::<u8>(),
            core::mem::size_of_val(urbs),
            BufferAttr::IN.or(BufferAttr::HIPC_AUTO_SELECT),
        )
    }

    /// CreateSmmuSpace (4.0.0+, cmd 7).
    ///
    /// Maps a buffer as device memory for use with transfer operations.
    /// Both `buffer` and `size` must be 0x1000-byte aligned.
    #[inline]
    pub fn create_smmu_space(&self, size: u32, buffer: u64) -> Result<(), DispatchError> {
        cmif::ep_create_smmu_space(&self.0, size, buffer)
    }

    /// ShareReportRing (4.0.0+, cmd 8).
    ///
    /// Creates a shared transfer-memory ring buffer for reading transfer
    /// reports without IPC. `tmem_handle` is the transfer-memory handle.
    #[inline]
    pub fn share_report_ring(&self, size: u64, tmem_handle: u32) -> Result<(), DispatchError> {
        cmif::ep_share_report_ring(&self.0, size, tmem_handle)
    }
}

// ---------------------------------------------------------------------------
// connect_cmif
// ---------------------------------------------------------------------------

/// Connects to the `usb:hs` service using CMIF.
///
/// The caller is responsible for converting to domain mode and performing
/// the initialization sequence (BindClientProcess, GetInterfaceStateChangeEvent,
/// etc.) appropriate for their target firmware version.
pub fn connect_cmif(sm: &SmService) -> Result<UsbHsService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::new(handle, 0);

    Ok(UsbHsService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get usb:hs service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
