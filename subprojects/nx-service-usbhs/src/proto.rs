//! USB host stack (`usb:hs`) service protocol constants.
//!
//! Command IDs differ between pre-2.0.0 and 2.0.0+. Per IC-4, both sets are
//! exposed and the caller selects the appropriate variant.

use nx_sf::ServiceName;

/// Service name for the USB host stack service (`usb:hs`).
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("usb:hs");

// ---------------------------------------------------------------------------
// IUsbHsService (root) — pre-2.0.0
// ---------------------------------------------------------------------------

/// QueryAllInterfaces (pre-2.0.0).
pub const QUERY_ALL_INTERFACES_LEGACY: u32 = 0;

/// QueryAvailableInterfaces (pre-2.0.0).
pub const QUERY_AVAILABLE_INTERFACES_LEGACY: u32 = 1;

/// QueryAcquiredInterfaces (pre-2.0.0).
pub const QUERY_ACQUIRED_INTERFACES_LEGACY: u32 = 2;

/// CreateInterfaceAvailableEvent (pre-2.0.0).
pub const CREATE_INTERFACE_AVAILABLE_EVENT_LEGACY: u32 = 3;

/// DestroyInterfaceAvailableEvent (pre-2.0.0).
pub const DESTROY_INTERFACE_AVAILABLE_EVENT_LEGACY: u32 = 4;

/// GetInterfaceStateChangeEvent (pre-2.0.0).
pub const GET_INTERFACE_STATE_CHANGE_EVENT_LEGACY: u32 = 5;

/// AcquireUsbIf (pre-2.0.0).
pub const ACQUIRE_USB_IF_LEGACY: u32 = 6;

// ---------------------------------------------------------------------------
// IUsbHsService (root) — 2.0.0+
// ---------------------------------------------------------------------------

/// BindClientProcess (2.0.0+). Sends process handle as copy-handle.
pub const BIND_CLIENT_PROCESS: u32 = 0;

/// QueryAllInterfaces (2.0.0+).
pub const QUERY_ALL_INTERFACES: u32 = 1;

/// QueryAvailableInterfaces (2.0.0+).
pub const QUERY_AVAILABLE_INTERFACES: u32 = 2;

/// QueryAcquiredInterfaces (2.0.0+).
pub const QUERY_ACQUIRED_INTERFACES: u32 = 3;

/// CreateInterfaceAvailableEvent (2.0.0+).
pub const CREATE_INTERFACE_AVAILABLE_EVENT: u32 = 4;

/// DestroyInterfaceAvailableEvent (2.0.0+).
pub const DESTROY_INTERFACE_AVAILABLE_EVENT: u32 = 5;

/// GetInterfaceStateChangeEvent (2.0.0+).
pub const GET_INTERFACE_STATE_CHANGE_EVENT: u32 = 6;

/// AcquireUsbIf (2.0.0+).
pub const ACQUIRE_USB_IF: u32 = 7;

// ---------------------------------------------------------------------------
// IClientIfSession — version-independent
// ---------------------------------------------------------------------------

/// GetCtrlXferEvent (cmd 0, all versions).
pub const IF_GET_CTRL_XFER_EVENT: u32 = 0;

/// SetInterface (cmd 1).
pub const IF_SET_INTERFACE: u32 = 1;

/// GetInterface (cmd 2).
pub const IF_GET_INTERFACE: u32 = 2;

/// GetAlternateInterface (cmd 3).
pub const IF_GET_ALTERNATE_INTERFACE: u32 = 3;

/// ResetDevice (cmd 8).
pub const IF_RESET_DEVICE: u32 = 8;

// ---------------------------------------------------------------------------
// IClientIfSession — pre-2.0.0
// ---------------------------------------------------------------------------

/// OpenUsbEp (pre-2.0.0, cmd 4).
pub const IF_OPEN_USB_EP_LEGACY: u32 = 4;

/// GetCurrentFrame (pre-2.0.0, cmd 5).
pub const IF_GET_CURRENT_FRAME_LEGACY: u32 = 5;

/// SubmitControlRequest IN (pre-2.0.0, cmd 6).
pub const IF_SUBMIT_CONTROL_REQUEST_IN: u32 = 6;

/// SubmitControlRequest OUT (pre-2.0.0, cmd 7).
pub const IF_SUBMIT_CONTROL_REQUEST_OUT: u32 = 7;

// ---------------------------------------------------------------------------
// IClientIfSession — 2.0.0+
// ---------------------------------------------------------------------------

/// GetCurrentFrame (2.0.0+, cmd 4).
pub const IF_GET_CURRENT_FRAME: u32 = 4;

/// CtrlXferAsync (2.0.0+, cmd 5).
pub const IF_CTRL_XFER_ASYNC: u32 = 5;

/// GetCtrlXferCompletionEvent (2.0.0+, cmd 6). Copy-handle out.
pub const IF_GET_CTRL_XFER_COMPLETION_EVENT: u32 = 6;

/// GetCtrlXferReport (2.0.0+, cmd 7).
pub const IF_GET_CTRL_XFER_REPORT: u32 = 7;

/// OpenUsbEp (2.0.0+, cmd 9).
pub const IF_OPEN_USB_EP: u32 = 9;

// ---------------------------------------------------------------------------
// IClientEpSession — pre-2.0.0
// ---------------------------------------------------------------------------

/// SubmitRequest OUT (pre-2.0.0, cmd 0).
pub const EP_SUBMIT_REQUEST_OUT: u32 = 0;

/// SubmitRequest IN (pre-2.0.0, cmd 1).
pub const EP_SUBMIT_REQUEST_IN: u32 = 1;

/// Close (pre-2.0.0, cmd 3).
pub const EP_CLOSE_LEGACY: u32 = 3;

// ---------------------------------------------------------------------------
// IClientEpSession — 2.0.0+
// ---------------------------------------------------------------------------

/// Close (2.0.0+, cmd 1).
pub const EP_CLOSE: u32 = 1;

/// GetXferEvent (2.0.0+, cmd 2). Copy-handle out.
pub const EP_GET_XFER_EVENT: u32 = 2;

/// Populate (2.0.0+, cmd 3).
pub const EP_POPULATE: u32 = 3;

/// PostBufferAsync (2.0.0+, cmd 4).
pub const EP_POST_BUFFER_ASYNC: u32 = 4;

/// GetXferReport (2.0.0+, cmd 5).
pub const EP_GET_XFER_REPORT: u32 = 5;

/// BatchBufferAsync (2.0.0+, cmd 6).
pub const EP_BATCH_BUFFER_ASYNC: u32 = 6;

/// CreateSmmuSpace (4.0.0+, cmd 7).
pub const EP_CREATE_SMMU_SPACE: u32 = 7;

/// ShareReportRing (4.0.0+, cmd 8).
pub const EP_SHARE_REPORT_RING: u32 = 8;
