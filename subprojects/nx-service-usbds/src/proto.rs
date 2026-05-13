//! USB device stack (`usb:ds`) service protocol constants.
//!
//! Command IDs are split by hosversion: pre-11.0.0 (legacy) and 11.0.0+.
//! Per IC-4, both sets are exposed and the caller selects.

use nx_sf::ServiceName;

/// Service name for the USB device stack service (`usb:ds`).
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("usb:ds");

// IDsService (root) — pre-11.0.0

/// BindDevice (pre-11.0.0).
pub const BIND_DEVICE_LEGACY: u32 = 0;

/// SetProcessHandle (pre-11.0.0, separate from BindDevice).
pub const SET_PROCESS_HANDLE_LEGACY: u32 = 1;

/// GetDsInterface / RegisterInterface (pre-11.0.0, 5.0.0+).
pub const GET_DS_INTERFACE_LEGACY: u32 = 2;

/// GetStateChangeEvent (pre-11.0.0).
pub const GET_STATE_CHANGE_EVENT_LEGACY: u32 = 3;

/// GetState (pre-11.0.0).
pub const GET_STATE_LEGACY: u32 = 4;

/// SetVidPidBcd (pre-5.0.0).
pub const SET_VID_PID_BCD: u32 = 5;

/// ClearDeviceData (5.0.0–10.x).
pub const CLEAR_DEVICE_DATA_LEGACY: u32 = 5;

/// AddUsbStringDescriptor (5.0.0–10.x).
pub const ADD_USB_STRING_DESCRIPTOR_LEGACY: u32 = 6;

/// DeleteUsbStringDescriptor (5.0.0–10.x).
pub const DELETE_USB_STRING_DESCRIPTOR_LEGACY: u32 = 7;

/// SetUsbDeviceDescriptor (5.0.0–10.x).
pub const SET_USB_DEVICE_DESCRIPTOR_LEGACY: u32 = 8;

/// SetBinaryObjectStore (5.0.0–10.x).
pub const SET_BINARY_OBJECT_STORE_LEGACY: u32 = 9;

/// Enable (5.0.0–10.x).
pub const ENABLE_LEGACY: u32 = 10;

/// Disable (5.0.0–10.x).
pub const DISABLE_LEGACY: u32 = 11;

/// GetSpeed (8.0.0–10.x).
pub const GET_SPEED_LEGACY: u32 = 12;

// IDsService (root) — 11.0.0+

/// BindDevice (11.0.0+, sends process handle inline).
pub const BIND_DEVICE: u32 = 0;

/// RegisterInterface (11.0.0+).
pub const REGISTER_INTERFACE: u32 = 1;

/// GetStateChangeEvent (11.0.0+).
pub const GET_STATE_CHANGE_EVENT: u32 = 2;

/// GetState (11.0.0+).
pub const GET_STATE: u32 = 3;

/// ClearDeviceData (11.0.0+).
pub const CLEAR_DEVICE_DATA: u32 = 4;

/// AddUsbStringDescriptor (11.0.0+).
pub const ADD_USB_STRING_DESCRIPTOR: u32 = 5;

/// DeleteUsbStringDescriptor (11.0.0+).
pub const DELETE_USB_STRING_DESCRIPTOR: u32 = 6;

/// SetUsbDeviceDescriptor (11.0.0+).
pub const SET_USB_DEVICE_DESCRIPTOR: u32 = 7;

/// SetBinaryObjectStore (11.0.0+).
pub const SET_BINARY_OBJECT_STORE: u32 = 8;

/// Enable (11.0.0+).
pub const ENABLE: u32 = 9;

/// Disable (11.0.0+).
pub const DISABLE: u32 = 10;

/// GetSpeed (11.0.0+).
pub const GET_SPEED: u32 = 11;

// IDsInterface — pre-11.0.0

/// RegisterEndpoint / GetDsEndpoint (pre-5.0.0).
pub const INTF_REGISTER_ENDPOINT: u32 = 0;

/// GetSetupEvent.
pub const INTF_GET_SETUP_EVENT: u32 = 1;

/// GetSetupPacket.
pub const INTF_GET_SETUP_PACKET: u32 = 2;

/// EnableInterface (pre-11.0.0).
pub const INTF_ENABLE_INTERFACE_LEGACY: u32 = 3;

/// DisableInterface (pre-11.0.0).
pub const INTF_DISABLE_INTERFACE_LEGACY: u32 = 4;

/// CtrlInPostBufferAsync (pre-11.0.0).
pub const INTF_CTRL_IN_POST_BUFFER_LEGACY: u32 = 5;

/// CtrlOutPostBufferAsync (pre-11.0.0).
pub const INTF_CTRL_OUT_POST_BUFFER_LEGACY: u32 = 6;

/// GetCtrlInCompletionEvent (pre-11.0.0).
pub const INTF_GET_CTRL_IN_COMPLETION_EVENT_LEGACY: u32 = 7;

/// GetCtrlInReportData (pre-11.0.0).
pub const INTF_GET_CTRL_IN_REPORT_DATA_LEGACY: u32 = 8;

/// GetCtrlOutCompletionEvent (pre-11.0.0).
pub const INTF_GET_CTRL_OUT_COMPLETION_EVENT_LEGACY: u32 = 9;

/// GetCtrlOutReportData (pre-11.0.0).
pub const INTF_GET_CTRL_OUT_REPORT_DATA_LEGACY: u32 = 10;

/// StallCtrl (pre-11.0.0).
pub const INTF_STALL_CTRL_LEGACY: u32 = 11;

/// AppendConfigurationData (5.0.0–10.x).
pub const INTF_APPEND_CONFIGURATION_DATA_LEGACY: u32 = 12;

// IDsInterface — 11.0.0+

/// CtrlInPostBufferAsync (11.0.0+).
pub const INTF_CTRL_IN_POST_BUFFER: u32 = 3;

/// CtrlOutPostBufferAsync (11.0.0+).
pub const INTF_CTRL_OUT_POST_BUFFER: u32 = 4;

/// GetCtrlInCompletionEvent (11.0.0+).
pub const INTF_GET_CTRL_IN_COMPLETION_EVENT: u32 = 5;

/// GetCtrlInReportData (11.0.0+).
pub const INTF_GET_CTRL_IN_REPORT_DATA: u32 = 6;

/// GetCtrlOutCompletionEvent (11.0.0+).
pub const INTF_GET_CTRL_OUT_COMPLETION_EVENT: u32 = 7;

/// GetCtrlOutReportData (11.0.0+).
pub const INTF_GET_CTRL_OUT_REPORT_DATA: u32 = 8;

/// StallCtrl (11.0.0+).
pub const INTF_STALL_CTRL: u32 = 9;

/// AppendConfigurationData (11.0.0+).
pub const INTF_APPEND_CONFIGURATION_DATA: u32 = 10;

// IDsEndpoint (version-independent)

/// PostBufferAsync.
pub const EP_POST_BUFFER_ASYNC: u32 = 0;

/// Cancel.
pub const EP_CANCEL: u32 = 1;

/// GetCompletionEvent.
pub const EP_GET_COMPLETION_EVENT: u32 = 2;

/// GetReportData.
pub const EP_GET_REPORT_DATA: u32 = 3;

/// Stall.
pub const EP_STALL: u32 = 4;

/// SetZlt.
pub const EP_SET_ZLT: u32 = 5;
