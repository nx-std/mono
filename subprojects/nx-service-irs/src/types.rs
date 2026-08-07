//! Wire-layout types for the IRS service.

use core::mem::size_of;

use static_assertions::const_assert_eq;

/// Maximum number of IR cameras (controller slots).
pub const MAX_CAMERAS: usize = 9;

/// IR camera availability status (from shared memory).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum IrCameraStatus {
    Available = 0,
    Unsupported = 1,
    Unconnected = 2,
}

/// IR camera internal status (from shared memory).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum IrCameraInternalStatus {
    Stopped = 0,
    FirmwareUpdateNeeded = 1,
    Unknown2 = 2,
    Unknown3 = 3,
    Unknown4 = 4,
    FirmwareVersionRequested = 5,
    FirmwareVersionIsInvalid = 6,
    /// \[4.0.0+\]
    Ready = 7,
    /// \[4.0.0+\]
    Setting = 8,
}

/// IR sensor operating mode (from shared memory).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum IrSensorMode {
    None = 0,
    MomentProcessor = 1,
    ClusteringProcessor = 2,
    ImageTransferProcessor = 3,
    PointingProcessor = 4,
    TeraPluginProcessor = 5,
    /// Does not appear in `DeviceFormat::ir_sensor_mode`.
    IrLedProcessor = 6,
}

/// Image processor status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ImageProcessorStatus {
    Stopped = 0,
    Running = 1,
}

/// Image transfer processor resolution format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ImageTransferProcessorFormat {
    /// 320x240
    Res320x240 = 0,
    /// 160x120
    Res160x120 = 1,
    /// 80x60
    Res80x60 = 2,
    /// 40x30 \[4.0.0+\]
    Res40x30 = 3,
    /// 20x15 \[4.0.0+\]
    Res20x15 = 4,
}

/// Adaptive clustering mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AdaptiveClusteringMode {
    StaticFov = 0,
    DynamicFov = 1,
}

/// Adaptive clustering target distance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AdaptiveClusteringTargetDistance {
    Near = 0,
    Middle = 1,
    Far = 2,
}

/// Hand analysis mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum HandAnalysisMode {
    Silhouette = 1,
    Image = 2,
    SilhouetteAndImage = 3,
    /// \[4.0.0+\]
    SilhouetteOnly = 4,
}

/// IR camera handle identifying a specific controller's IR sensor.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct IrCameraHandle {
    pub player_number: u8,
    pub device_type: u8,
    pub reserved: [u8; 2],
}
const_assert_eq!(size_of::<IrCameraHandle>(), 0x4);

/// Packed MCU firmware version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct PackedMcuVersion {
    pub major_version: u16,
    pub minor_version: u16,
}
const_assert_eq!(size_of::<PackedMcuVersion>(), 0x4);

/// Packed function level for IR sensor activation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct PackedFunctionLevel {
    pub ir_sensor_function_level: u8,
    pub reserved: [u8; 3],
}
const_assert_eq!(size_of::<PackedFunctionLevel>(), 0x4);

/// Rectangle for window-of-interest regions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct Rect {
    pub x: i16,
    pub y: i16,
    pub width: i16,
    pub height: i16,
}
const_assert_eq!(size_of::<Rect>(), 0x8);

/// Packed wire-layout config for MomentProcessor (cmd 306).
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct PackedMomentProcessorConfig {
    pub exposure_time: u64,
    pub light_target: u8,
    pub gain: u8,
    pub is_negative_image_used: u8,
    pub reserved: [u8; 5],
    pub window_of_interest: Rect,
    pub required_mcu_version: PackedMcuVersion,
    pub preprocess: u8,
    pub preprocess_intensity_threshold: u8,
    pub reserved2: [u8; 2],
}
const_assert_eq!(size_of::<PackedMomentProcessorConfig>(), 0x20);

/// Packed wire-layout config for ClusteringProcessor (cmd 307).
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct PackedClusteringProcessorConfig {
    pub exposure_time: u64,
    pub light_target: u8,
    pub gain: u8,
    pub is_negative_image_used: u8,
    pub reserved: [u8; 5],
    pub window_of_interest: Rect,
    pub required_mcu_version: PackedMcuVersion,
    pub object_pixel_count_min: u32,
    pub object_pixel_count_max: u32,
    pub object_intensity_min: u8,
    pub is_external_light_filter_enabled: u8,
    pub reserved2: [u8; 2],
}
const_assert_eq!(size_of::<PackedClusteringProcessorConfig>(), 0x28);

/// Packed wire-layout config for ImageTransferProcessor (cmd 308).
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct PackedImageTransferProcessorConfig {
    pub exposure_time: u64,
    pub light_target: u8,
    pub gain: u8,
    pub is_negative_image_used: u8,
    pub reserved: [u8; 5],
    pub required_mcu_version: PackedMcuVersion,
    pub format: u8,
    pub reserved2: [u8; 3],
}
const_assert_eq!(size_of::<PackedImageTransferProcessorConfig>(), 0x18);

/// Packed wire-layout config for ImageTransferExProcessor (cmd 316). \[4.0.0+\]
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct PackedImageTransferProcessorExConfig {
    pub exposure_time: u64,
    pub light_target: u8,
    pub gain: u8,
    pub is_negative_image_used: u8,
    pub reserved: [u8; 5],
    pub required_mcu_version: PackedMcuVersion,
    pub orig_format: u8,
    pub trimming_format: u8,
    pub trimming_start_x: u16,
    pub trimming_start_y: u16,
    pub is_external_light_filter_enabled: u8,
    pub reserved2: [u8; 5],
}
const_assert_eq!(size_of::<PackedImageTransferProcessorExConfig>(), 0x20);

/// Packed wire-layout config for PointingProcessor (cmd 312).
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct PackedPointingProcessorConfig {
    pub window_of_interest: Rect,
    pub required_mcu_version: PackedMcuVersion,
}
const_assert_eq!(size_of::<PackedPointingProcessorConfig>(), 0xC);

/// Packed wire-layout config for TeraPluginProcessor (cmd 310).
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct PackedTeraPluginProcessorConfig {
    pub required_mcu_version: PackedMcuVersion,
    pub mode: u8,
    pub unk_x5: u8,
    pub unk_x6: u8,
    pub unk_x7: u8,
}
const_assert_eq!(size_of::<PackedTeraPluginProcessorConfig>(), 0x8);

/// Packed wire-layout config for IrLedProcessor (cmd 317). \[4.0.0+\]
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct PackedIrLedProcessorConfig {
    pub required_mcu_version: PackedMcuVersion,
    pub light_target: u8,
    pub pad: [u8; 3],
}
const_assert_eq!(size_of::<PackedIrLedProcessorConfig>(), 0x8);

/// Input for StopImageProcessor / SuspendImageProcessor / StopImageProcessorAsync
/// (cmds 305/313/318).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct HandleAruidIn {
    pub handle: IrCameraHandle,
    pub pad: u32,
    pub applet_resource_user_id: u64,
}
const_assert_eq!(size_of::<HandleAruidIn>(), 0x10);

/// Input for RunMomentProcessor (cmd 306).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct RunMomentProcessorIn {
    pub handle: IrCameraHandle,
    pub pad: u32,
    pub applet_resource_user_id: u64,
    pub config: PackedMomentProcessorConfig,
}
const_assert_eq!(size_of::<RunMomentProcessorIn>(), 0x30);

/// Input for RunClusteringProcessor (cmd 307).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct RunClusteringProcessorIn {
    pub handle: IrCameraHandle,
    pub pad: u32,
    pub applet_resource_user_id: u64,
    pub config: PackedClusteringProcessorConfig,
}
const_assert_eq!(size_of::<RunClusteringProcessorIn>(), 0x38);

/// Input for RunImageTransferProcessor (cmd 308).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct RunImageTransferProcessorIn {
    pub handle: IrCameraHandle,
    pub pad: u32,
    pub applet_resource_user_id: u64,
    pub config: PackedImageTransferProcessorConfig,
    pub transfer_memory_size: u64,
}
const_assert_eq!(size_of::<RunImageTransferProcessorIn>(), 0x30);

/// Input for GetImageTransferProcessorState (cmd 309).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct GetImageTransferProcessorStateIn {
    pub handle: IrCameraHandle,
    pub pad: u32,
    pub applet_resource_user_id: u64,
}
const_assert_eq!(size_of::<GetImageTransferProcessorStateIn>(), 0x10);

/// Input for RunTeraPluginProcessor (cmd 310).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct RunTeraPluginProcessorIn {
    pub handle: IrCameraHandle,
    pub config: PackedTeraPluginProcessorConfig,
    /// Alignment slack ahead of the 8-byte-aligned ARUID.
    pub pad: u32,
    pub applet_resource_user_id: u64,
}
const_assert_eq!(size_of::<RunTeraPluginProcessorIn>(), 0x18);

/// Input for RunPointingProcessor (cmd 312).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct RunPointingProcessorIn {
    pub handle: IrCameraHandle,
    pub config: PackedPointingProcessorConfig,
    pub applet_resource_user_id: u64,
}
const_assert_eq!(size_of::<RunPointingProcessorIn>(), 0x18);

/// Input for CheckFirmwareVersion (cmd 314). \[3.0.0+\]
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct CheckFirmwareVersionIn {
    pub handle: IrCameraHandle,
    pub version: PackedMcuVersion,
    pub pad: u32,
    /// Alignment slack ahead of the 8-byte-aligned ARUID.
    pub pad2: u32,
    pub applet_resource_user_id: u64,
}
const_assert_eq!(size_of::<CheckFirmwareVersionIn>(), 0x18);

/// Input for RunImageTransferExProcessor (cmd 316). \[4.0.0+\]
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct RunImageTransferExProcessorIn {
    pub handle: IrCameraHandle,
    pub pad: u32,
    pub applet_resource_user_id: u64,
    pub config: PackedImageTransferProcessorExConfig,
    pub transfer_memory_size: u64,
}
const_assert_eq!(size_of::<RunImageTransferExProcessorIn>(), 0x38);

/// Input for RunIrLedProcessor (cmd 317). \[4.0.0+\]
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct RunIrLedProcessorIn {
    pub handle: IrCameraHandle,
    pub config: PackedIrLedProcessorConfig,
    /// Alignment slack ahead of the 8-byte-aligned ARUID.
    pub pad: u32,
    pub applet_resource_user_id: u64,
}
const_assert_eq!(size_of::<RunIrLedProcessorIn>(), 0x18);

/// Input for ActivateIrsensorWithFunctionLevel (cmd 319). \[4.0.0+\]
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct ActivateWithFunctionLevelIn {
    pub level: PackedFunctionLevel,
    pub pad: u32,
    pub applet_resource_user_id: u64,
}
const_assert_eq!(size_of::<ActivateWithFunctionLevelIn>(), 0x10);

/// Single statistic entry from the moment processor.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MomentStatistic {
    pub average_intensity: f32,
    pub centroid_x: f32,
    pub centroid_y: f32,
}
const_assert_eq!(size_of::<MomentStatistic>(), 0xC);

/// State entry from the moment processor (shared memory ring buffer).
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MomentProcessorState {
    pub sampling_number: i64,
    pub timestamp: u64,
    pub ambient_noise_level: u32,
    pub reserved: [u8; 4],
    pub statistic: [MomentStatistic; 0x30],
}
const_assert_eq!(size_of::<MomentProcessorState>(), 0x258);

/// Single clustering data entry.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ClusteringData {
    pub average_intensity: f32,
    pub centroid_x: f32,
    pub centroid_y: f32,
    pub pixel_count: u32,
    pub bound_x: u16,
    pub bound_y: u16,
    pub bound_width: u16,
    pub bound_height: u16,
}
const_assert_eq!(size_of::<ClusteringData>(), 0x18);

/// State entry from the clustering processor (shared memory ring buffer).
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ClusteringProcessorState {
    pub sampling_number: i64,
    pub timestamp: u64,
    pub object_count: u8,
    pub reserved: [u8; 3],
    pub ambient_noise_level: u32,
    pub data: [ClusteringData; 0x10],
}
const_assert_eq!(size_of::<ClusteringProcessorState>(), 0x198);

/// State returned by GetImageTransferProcessorState (cmd 309 output).
#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub struct ImageTransferProcessorState {
    pub sampling_number: u64,
    pub ambient_noise_level: u32,
    pub reserved: [u8; 4],
}
const_assert_eq!(size_of::<ImageTransferProcessorState>(), 0x10);

/// Single marker data entry for the pointing processor.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct PointingProcessorMarkerData {
    pub pointing_status: u8,
    pub reserved: [u8; 3],
    pub unk_x4: [u8; 4],
    pub unk_x8: f32,
    pub position_x: f32,
    pub position_y: f32,
    pub unk_x14: f32,
    pub window_of_interest: Rect,
}
const_assert_eq!(size_of::<PointingProcessorMarkerData>(), 0x20);

/// State entry from the pointing processor marker (shared memory ring buffer).
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct PointingProcessorMarkerState {
    pub sampling_number: i64,
    pub timestamp: u64,
    pub data: [PointingProcessorMarkerData; 3],
}
const_assert_eq!(size_of::<PointingProcessorMarkerState>(), 0x70);

/// Simplified pointing processor state.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct PointingProcessorState {
    pub sampling_number: i64,
    pub timestamp: u64,
    pub pointing_status: u32,
    pub position_x: f32,
    pub position_y: f32,
    pub reserved: [u8; 4],
}
const_assert_eq!(size_of::<PointingProcessorState>(), 0x20);

/// State entry from the tera-plugin processor (shared memory ring buffer).
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TeraPluginProcessorState {
    pub sampling_number: i64,
    pub timestamp: u64,
    pub ambient_noise_level: u32,
    pub plugin_data: [u8; 0x12C],
}
const_assert_eq!(size_of::<TeraPluginProcessorState>(), 0x140);

/// Ring-buffer header for processor state entries in shared memory.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ProcessorState {
    pub start: i64,
    pub count: u32,
    pub pad: u32,
    pub data: [u8; 0xE10],
}
const_assert_eq!(size_of::<ProcessorState>(), 0xE20);

/// Per-camera device format in shared memory.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct DeviceFormat {
    pub ir_camera_status: u32,
    pub ir_camera_internal_status: u32,
    pub ir_sensor_mode: u32,
    pub pad: u32,
    pub processor_state: ProcessorState,
}
const_assert_eq!(size_of::<DeviceFormat>(), 0xE30);

/// Per-ARUID format in shared memory.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct AruidFormat {
    pub ir_sensor_aruid: u64,
    pub ir_sensor_aruid_status: u32,
    pub pad: u32,
}
const_assert_eq!(size_of::<AruidFormat>(), 0x10);

/// Top-level shared memory layout (0x8000 bytes, mapped read-only).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct StatusManager {
    pub device_format: [DeviceFormat; MAX_CAMERAS],
    pub aruid_format: [AruidFormat; 5],
}
const_assert_eq!(size_of::<StatusManager>(), 0x8000);

/// User-facing config for MomentProcessor (wider fields than the packed wire
/// format).
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MomentProcessorConfig {
    pub exposure_time: u64,
    pub light_target: u32,
    pub gain: u32,
    pub is_negative_image_used: u8,
    pub reserved: [u8; 7],
    pub window_of_interest: Rect,
    pub preprocess: u32,
    pub preprocess_intensity_threshold: u32,
}
const_assert_eq!(size_of::<MomentProcessorConfig>(), 0x28);

/// User-facing config for ClusteringProcessor.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ClusteringProcessorConfig {
    pub exposure_time: u64,
    pub light_target: u32,
    pub gain: u32,
    pub is_negative_image_used: u8,
    pub reserved: [u8; 7],
    pub window_of_interest: Rect,
    pub object_pixel_count_min: u32,
    pub object_pixel_count_max: u32,
    pub object_intensity_min: u32,
    pub is_external_light_filter_enabled: u8,
}
const_assert_eq!(size_of::<ClusteringProcessorConfig>(), 0x30);

/// User-facing config for ImageTransferProcessor.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ImageTransferProcessorConfig {
    pub exposure_time: u64,
    pub light_target: u32,
    pub gain: u32,
    pub is_negative_image_used: u8,
    pub reserved: [u8; 7],
    pub format: u32,
}
const_assert_eq!(size_of::<ImageTransferProcessorConfig>(), 0x20);

/// User-facing config for ImageTransferExProcessor. \[4.0.0+\]
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ImageTransferProcessorExConfig {
    pub exposure_time: u64,
    pub light_target: u32,
    pub gain: u32,
    pub is_negative_image_used: u8,
    pub reserved: [u8; 7],
    pub orig_format: u32,
    pub trimming_format: u32,
    pub trimming_start_x: u16,
    pub trimming_start_y: u16,
    pub is_external_light_filter_enabled: u8,
}
const_assert_eq!(size_of::<ImageTransferProcessorExConfig>(), 0x28);

/// User-facing config for TeraPluginProcessor.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TeraPluginProcessorConfig {
    pub mode: u8,
    /// \[6.0.0+\]
    pub unk_x1: u8,
    /// \[6.0.0+\]
    pub unk_x2: u8,
    /// \[6.0.0+\]
    pub unk_x3: u8,
}
const_assert_eq!(size_of::<TeraPluginProcessorConfig>(), 0x4);

/// User-facing config for IrLedProcessor. \[4.0.0+\]
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct IrLedProcessorConfig {
    pub light_target: u32,
}
const_assert_eq!(size_of::<IrLedProcessorConfig>(), 0x4);

/// User-facing config for AdaptiveClusteringProcessor. \[5.0.0+\]
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct AdaptiveClusteringProcessorConfig {
    pub mode: u32,
    /// \[6.0.0+\]
    pub target_distance: u32,
}
const_assert_eq!(size_of::<AdaptiveClusteringProcessorConfig>(), 0x8);

/// User-facing config for HandAnalysis.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct HandAnalysisConfig {
    pub mode: u32,
}
const_assert_eq!(size_of::<HandAnalysisConfig>(), 0x4);
