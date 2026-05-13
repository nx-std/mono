//! HID Bus service protocol constants.

use nx_sf::ServiceName;

/// Service name for the HID Bus service.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("hidbus");

/// GetBusHandle (cmd 1).
pub const GET_BUS_HANDLE: u32 = 1;

/// Initialize (cmd 3).
pub const INITIALIZE: u32 = 3;

/// Finalize (cmd 4).
pub const FINALIZE: u32 = 4;

/// EnableExternalDevice (cmd 5).
pub const ENABLE_EXTERNAL_DEVICE: u32 = 5;

/// GetExternalDeviceId (cmd 6).
pub const GET_EXTERNAL_DEVICE_ID: u32 = 6;

/// SendCommandAsync (cmd 7).
pub const SEND_COMMAND_ASYNC: u32 = 7;

/// GetSendCommandAsyncResult (cmd 8).
pub const GET_SEND_COMMAND_ASYNC_RESULT: u32 = 8;

/// SetEventForSendCommandAsyncResult (cmd 9).
pub const SET_EVENT_FOR_SEND_COMMAND_ASYNC_RESULT: u32 = 9;

/// GetSharedMemoryHandle (cmd 10).
pub const GET_SHARED_MEMORY_HANDLE: u32 = 10;

/// EnableJoyPollingReceiveMode (cmd 11).
pub const ENABLE_JOY_POLLING_RECEIVE_MODE: u32 = 11;

/// DisableJoyPollingReceiveMode (cmd 12).
pub const DISABLE_JOY_POLLING_RECEIVE_MODE: u32 = 12;

/// SetStatusManagerType (cmd 14).
pub const SET_STATUS_MANAGER_TYPE: u32 = 14;
