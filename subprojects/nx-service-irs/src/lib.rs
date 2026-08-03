//! HID IR sensor (`irs`) service implementation.
//!
//! Provides access to the IR camera sensors on Joy-Con controllers.
//! The service is non-domain and most commands send PID +
//! `AppletResourceUserId`.
//!
//! ## Usage
//!
//! 1. Connect via [`connect_cmif`].
//! 2. Activate the sensor with [`IrsService::activate_irsensor`] (pre-4.0.0)
//!    or [`IrsService::activate_irsensor_with_function_level`] (4.0.0+).
//! 3. Obtain the shared memory handle via
//!    [`IrsService::get_irsensor_shared_memory_handle`] and map it.
//! 4. Get a camera handle with [`IrsService::get_ir_camera_handle`].
//! 5. Run a processor (moment, clustering, image transfer, etc.).
//! 6. Read state from shared memory ([`StatusManager`]).
//! 7. Deactivate with [`IrsService::deactivate_irsensor`] when done.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::{BorrowedSessionHandle, DispatchError, Session};

mod cmif;
mod dispatch;
mod proto;
mod types;

pub use self::{
    cmif::GetSharedMemoryError,
    proto::SERVICE_NAME,
    types::{
        AdaptiveClusteringMode, AdaptiveClusteringProcessorConfig,
        AdaptiveClusteringTargetDistance, AruidFormat, ClusteringData, ClusteringProcessorConfig,
        ClusteringProcessorState, DeviceFormat, HandAnalysisConfig, HandAnalysisMode,
        ImageProcessorStatus, ImageTransferProcessorConfig, ImageTransferProcessorExConfig,
        ImageTransferProcessorFormat, ImageTransferProcessorState, IrCameraHandle,
        IrCameraInternalStatus, IrCameraStatus, IrLedProcessorConfig, IrSensorMode, MAX_CAMERAS,
        MomentProcessorConfig, MomentProcessorState, MomentStatistic,
        PackedClusteringProcessorConfig, PackedFunctionLevel, PackedImageTransferProcessorConfig,
        PackedImageTransferProcessorExConfig, PackedIrLedProcessorConfig, PackedMcuVersion,
        PackedMomentProcessorConfig, PackedPointingProcessorConfig,
        PackedTeraPluginProcessorConfig, PointingProcessorMarkerData, PointingProcessorMarkerState,
        PointingProcessorState, ProcessorState, Rect, StatusManager, TeraPluginProcessorConfig,
        TeraPluginProcessorState,
    },
};

/// IRS service wrapper.
#[repr(transparent)]
pub struct IrsService(Session);

impl IrsService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

// ---------------------------------------------------------------------------
// Activation / deactivation
// ---------------------------------------------------------------------------

impl IrsService {
    /// Activates the IR sensor (cmd 302, pre-4.0.0).
    #[inline]
    pub fn activate_irsensor(&self, applet_resource_user_id: u64) -> Result<(), DispatchError> {
        cmif::activate_irsensor(&self.0, applet_resource_user_id)
    }

    /// Deactivates the IR sensor (cmd 303).
    #[inline]
    pub fn deactivate_irsensor(&self, applet_resource_user_id: u64) -> Result<(), DispatchError> {
        cmif::deactivate_irsensor(&self.0, applet_resource_user_id)
    }

    /// Gets the shared memory handle for the IR sensor status manager
    /// (cmd 304). Returns a copy handle that the caller must map.
    #[inline]
    pub fn get_irsensor_shared_memory_handle(
        &self,
        applet_resource_user_id: u64,
    ) -> Result<u32, GetSharedMemoryError> {
        cmif::get_irsensor_shared_memory_handle(&self.0, applet_resource_user_id)
    }
}

// ---------------------------------------------------------------------------
// Camera handle
// ---------------------------------------------------------------------------

impl IrsService {
    /// Gets an IR camera handle for the given NpadIdType (cmd 311).
    #[inline]
    pub fn get_ir_camera_handle(&self, npad_id: u32) -> Result<IrCameraHandle, DispatchError> {
        cmif::get_ir_camera_handle(&self.0, npad_id)
    }
}

// ---------------------------------------------------------------------------
// Processor lifecycle
// ---------------------------------------------------------------------------

impl IrsService {
    /// Stops the image processor (cmd 305).
    #[inline]
    pub fn stop_image_processor(
        &self,
        handle: IrCameraHandle,
        applet_resource_user_id: u64,
    ) -> Result<(), DispatchError> {
        cmif::stop_image_processor(&self.0, handle, applet_resource_user_id)
    }

    /// Suspends the image processor (cmd 313).
    #[inline]
    pub fn suspend_image_processor(
        &self,
        handle: IrCameraHandle,
        applet_resource_user_id: u64,
    ) -> Result<(), DispatchError> {
        cmif::suspend_image_processor(&self.0, handle, applet_resource_user_id)
    }

    /// Stops the image processor asynchronously (cmd 318). \[4.0.0+\]
    #[inline]
    pub fn stop_image_processor_async(
        &self,
        handle: IrCameraHandle,
        applet_resource_user_id: u64,
    ) -> Result<(), DispatchError> {
        cmif::stop_image_processor_async(&self.0, handle, applet_resource_user_id)
    }
}

// ---------------------------------------------------------------------------
// Run processors
// ---------------------------------------------------------------------------

impl IrsService {
    /// Runs the moment processor (cmd 306).
    #[inline]
    pub fn run_moment_processor(
        &self,
        handle: IrCameraHandle,
        applet_resource_user_id: u64,
        config: &PackedMomentProcessorConfig,
    ) -> Result<(), DispatchError> {
        cmif::run_moment_processor(&self.0, handle, applet_resource_user_id, config)
    }

    /// Runs the clustering processor (cmd 307).
    #[inline]
    pub fn run_clustering_processor(
        &self,
        handle: IrCameraHandle,
        applet_resource_user_id: u64,
        config: &PackedClusteringProcessorConfig,
    ) -> Result<(), DispatchError> {
        cmif::run_clustering_processor(&self.0, handle, applet_resource_user_id, config)
    }

    /// Runs the image transfer processor (cmd 308). The caller must create a
    /// transfer memory region and pass its handle and size.
    #[inline]
    pub fn run_image_transfer_processor(
        &self,
        handle: IrCameraHandle,
        applet_resource_user_id: u64,
        config: &PackedImageTransferProcessorConfig,
        transfer_memory_size: u64,
        tmem_handle: u32,
    ) -> Result<(), DispatchError> {
        cmif::run_image_transfer_processor(
            &self.0,
            handle,
            applet_resource_user_id,
            config,
            transfer_memory_size,
            tmem_handle,
        )
    }

    /// Gets the image transfer processor state (cmd 309). Fills `buffer` via
    /// HipcMapAlias and returns the state header.
    #[inline]
    pub fn get_image_transfer_processor_state(
        &self,
        handle: IrCameraHandle,
        applet_resource_user_id: u64,
        buffer: &mut [u8],
    ) -> Result<ImageTransferProcessorState, DispatchError> {
        cmif::get_image_transfer_processor_state(&self.0, handle, applet_resource_user_id, buffer)
    }

    /// Runs the tera-plugin processor (cmd 310).
    #[inline]
    pub fn run_tera_plugin_processor(
        &self,
        handle: IrCameraHandle,
        applet_resource_user_id: u64,
        config: &PackedTeraPluginProcessorConfig,
    ) -> Result<(), DispatchError> {
        cmif::run_tera_plugin_processor(&self.0, handle, applet_resource_user_id, config)
    }

    /// Runs the pointing processor (cmd 312).
    #[inline]
    pub fn run_pointing_processor(
        &self,
        handle: IrCameraHandle,
        applet_resource_user_id: u64,
        config: &PackedPointingProcessorConfig,
    ) -> Result<(), DispatchError> {
        cmif::run_pointing_processor(&self.0, handle, applet_resource_user_id, config)
    }

    /// Runs the image transfer ex processor (cmd 316). \[4.0.0+\] The caller
    /// must create a transfer memory region and pass its handle and size.
    #[inline]
    pub fn run_image_transfer_ex_processor(
        &self,
        handle: IrCameraHandle,
        applet_resource_user_id: u64,
        config: &PackedImageTransferProcessorExConfig,
        transfer_memory_size: u64,
        tmem_handle: u32,
    ) -> Result<(), DispatchError> {
        cmif::run_image_transfer_ex_processor(
            &self.0,
            handle,
            applet_resource_user_id,
            config,
            transfer_memory_size,
            tmem_handle,
        )
    }

    /// Runs the IR LED processor (cmd 317). \[4.0.0+\]
    #[inline]
    pub fn run_ir_led_processor(
        &self,
        handle: IrCameraHandle,
        applet_resource_user_id: u64,
        config: &PackedIrLedProcessorConfig,
    ) -> Result<(), DispatchError> {
        cmif::run_ir_led_processor(&self.0, handle, applet_resource_user_id, config)
    }
}

// ---------------------------------------------------------------------------
// Firmware
// ---------------------------------------------------------------------------

impl IrsService {
    /// Checks the firmware version (cmd 314). \[3.0.0+\]
    #[inline]
    pub fn check_firmware_version(
        &self,
        handle: IrCameraHandle,
        version: PackedMcuVersion,
        applet_resource_user_id: u64,
    ) -> Result<(), DispatchError> {
        cmif::check_firmware_version(&self.0, handle, version, applet_resource_user_id)
    }

    /// Activates the IR sensor with a function level (cmd 319). \[4.0.0+\]
    #[inline]
    pub fn activate_irsensor_with_function_level(
        &self,
        level: PackedFunctionLevel,
        applet_resource_user_id: u64,
    ) -> Result<(), DispatchError> {
        cmif::activate_irsensor_with_function_level(&self.0, level, applet_resource_user_id)
    }
}

// ---------------------------------------------------------------------------
// Connection
// ---------------------------------------------------------------------------

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get irs service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);

/// Connects to the `irs` service via SM and returns an [`IrsService`] wrapper.
pub fn connect_cmif(sm: &SmService) -> Result<IrsService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let session = Session::new(handle, 0);
    Ok(IrsService(session))
}
