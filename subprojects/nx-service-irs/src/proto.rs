//! IRS service protocol constants.

use nx_sf::ServiceName;

/// Service name for the IR sensor service.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("irs");

// ---------------------------------------------------------------------------
// Activation / deactivation
// ---------------------------------------------------------------------------

/// ActivateIrsensor (cmd 302).
pub const ACTIVATE_IRSENSOR: u32 = 302;

/// DeactivateIrsensor (cmd 303).
pub const DEACTIVATE_IRSENSOR: u32 = 303;

/// GetIrsensorSharedMemoryHandle (cmd 304).
pub const GET_IRSENSOR_SHARED_MEMORY_HANDLE: u32 = 304;

// ---------------------------------------------------------------------------
// Processor control
// ---------------------------------------------------------------------------

/// StopImageProcessor (cmd 305).
pub const STOP_IMAGE_PROCESSOR: u32 = 305;

/// RunMomentProcessor (cmd 306).
pub const RUN_MOMENT_PROCESSOR: u32 = 306;

/// RunClusteringProcessor (cmd 307).
pub const RUN_CLUSTERING_PROCESSOR: u32 = 307;

/// RunImageTransferProcessor (cmd 308).
pub const RUN_IMAGE_TRANSFER_PROCESSOR: u32 = 308;

/// GetImageTransferProcessorState (cmd 309).
pub const GET_IMAGE_TRANSFER_PROCESSOR_STATE: u32 = 309;

/// RunTeraPluginProcessor (cmd 310).
pub const RUN_TERA_PLUGIN_PROCESSOR: u32 = 310;

/// GetIrCameraHandle (cmd 311).
pub const GET_IR_CAMERA_HANDLE: u32 = 311;

/// RunPointingProcessor (cmd 312).
pub const RUN_POINTING_PROCESSOR: u32 = 312;

/// SuspendImageProcessor (cmd 313).
pub const SUSPEND_IMAGE_PROCESSOR: u32 = 313;

/// CheckFirmwareVersion (cmd 314). \[3.0.0+\]
pub const CHECK_FIRMWARE_VERSION: u32 = 314;

/// RunImageTransferExProcessor (cmd 316). \[4.0.0+\]
pub const RUN_IMAGE_TRANSFER_EX_PROCESSOR: u32 = 316;

/// RunIrLedProcessor (cmd 317). \[4.0.0+\]
pub const RUN_IR_LED_PROCESSOR: u32 = 317;

/// StopImageProcessorAsync (cmd 318). \[4.0.0+\]
pub const STOP_IMAGE_PROCESSOR_ASYNC: u32 = 318;

/// ActivateIrsensorWithFunctionLevel (cmd 319). \[4.0.0+\]
pub const ACTIVATE_IRSENSOR_WITH_FUNCTION_LEVEL: u32 = 319;
