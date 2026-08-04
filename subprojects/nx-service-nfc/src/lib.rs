//! NFC, NFP (amiibo), and Mifare service implementations.
//!
//! This crate provides access to three related NFC services on the Nintendo Switch:
//!
//! - [`NfpService`] — Nintendo Figurine Platform (amiibo) via `nfp:user`, `nfp:dbg`,
//!   or `nfp:sys`.
//! - [`NfcService`] — NFC tag operations via `nfc:user` or `nfc:sys`.
//! - [`NfcMifareService`] — NFC Mifare operations via `nfc:mf:u`.
//!
//! All three are domain-mode services that create an interface sub-object during
//! initialization. The interface is initialized with PID, ARUID, and MCU version
//! data matching libnx's initialization pattern.
//!
//! ## Divergence from libnx
//!
//! libnx keeps guarded global singletons and calls `hosversionBefore` at runtime
//! to select command IDs. This crate is hosversion-unaware per IC-4: the NFC
//! service exposes paired `_legacy` / versioned method variants, and callers
//! select based on the target firmware. ARUID is caller-provided rather than
//! fetched internally from applet.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::{ConvertToDomainError, Domain, DomainObjectRef, Session};

mod cmif;
mod dispatch;
mod proto;
mod types;

pub use nx_sf::service::DispatchError;

pub use self::{
    proto::{
        NFC_MF_SERVICE_NAME, NFC_SYS_SERVICE_NAME, NFC_USER_SERVICE_NAME, NFP_DBG_SERVICE_NAME,
        NFP_SYS_SERVICE_NAME, NFP_USER_SERVICE_NAME,
    },
    types::{
        NfcDeviceHandle, NfcDeviceState, NfcMifareCommand, NfcMifareDeviceState,
        NfcMifareReadBlockData, NfcMifareReadBlockParameter, NfcMifareWriteBlockParameter,
        NfcProtocol, NfcRequiredMcuVersionData, NfcSectorKey, NfcServiceType, NfcState, NfcTagInfo,
        NfcTagType, NfpAdminInfo, NfpAmiiboFlag, NfpApplicationAreaVersion, NfpBreakType,
        NfpCommonInfo, NfpData, NfpDate, NfpDeviceState, NfpDeviceType, NfpModelInfo,
        NfpMountTarget, NfpRegisterInfo, NfpRegisterInfoPrivate, NfpServiceType, NfpTagInfo,
    },
};

/// Default MCU version data sent during initialization (matches libnx).
pub const DEFAULT_MCU_VERSION_DATA: [NfcRequiredMcuVersionData; 2] = [
    NfcRequiredMcuVersionData {
        version: 0x0000_0001_000a_0003,
        reserved: [0; 3],
    },
    NfcRequiredMcuVersionData {
        version: 0x0000_0003_0004_0003,
        reserved: [0; 3],
    },
];

/// Connected NFP (amiibo) service wrapper.
///
/// The service operates in domain mode; the interface sub-object is created
/// during [`connect_nfp_cmif`] and initialized with PID + ARUID + MCU version
/// data.
pub struct NfpService {
    domain: Domain,
    /// Object id of the interface sub-object inside `domain`.
    interface_id: u32,
}

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for NfpService {}
unsafe impl Sync for NfpService {}

impl NfpService {
    /// Addresses the interface sub-object inside the service's domain.
    ///
    /// Built on demand rather than stored: a stored view would have to name a
    /// lifetime that borrows the `domain` field beside it, which a struct
    /// cannot express. The view closes nothing, so the sub-object outlives
    /// every call made through it and `Drop` finalizes it exactly once.
    #[inline]
    fn interface(&self) -> DomainObjectRef<'_> {
        // SAFETY: `interface_id` was returned by `create_interface` on this
        // same domain at connect time and is closed only in `Drop`, so it
        // names a live server-side object for as long as `self` exists.
        DomainObjectRef::from_raw_unchecked(self.domain.as_borrowed(), self.interface_id)
            .expect("interface object id is non-zero once stored")
    }

    /// Lists connected NFC devices.
    ///
    /// Writes device handles into `out` and returns the number written.
    #[inline]
    pub fn list_devices(&self, out: &mut [NfcDeviceHandle]) -> Result<i32, DispatchError> {
        cmif::nfp::list_devices(self.interface(), out)
    }

    /// Starts NFC tag detection on the given device.
    #[inline]
    pub fn start_detection(&self, handle: &NfcDeviceHandle) -> Result<(), DispatchError> {
        cmif::nfp::start_detection(self.interface(), handle)
    }

    /// Stops NFC tag detection on the given device.
    #[inline]
    pub fn stop_detection(&self, handle: &NfcDeviceHandle) -> Result<(), DispatchError> {
        cmif::nfp::stop_detection(self.interface(), handle)
    }

    /// Mounts the amiibo tag for access.
    #[inline]
    pub fn mount(
        &self,
        handle: &NfcDeviceHandle,
        device_type: NfpDeviceType,
        mount_target: NfpMountTarget,
    ) -> Result<(), DispatchError> {
        cmif::nfp::mount(
            self.interface(),
            handle,
            device_type as u32,
            mount_target.bits(),
        )
    }

    /// Unmounts the amiibo tag.
    #[inline]
    pub fn unmount(&self, handle: &NfcDeviceHandle) -> Result<(), DispatchError> {
        cmif::nfp::unmount(self.interface(), handle)
    }

    /// Opens the application area for the mounted amiibo.
    #[inline]
    pub fn open_application_area(
        &self,
        handle: &NfcDeviceHandle,
        app_id: u32,
    ) -> Result<(), DispatchError> {
        cmif::nfp::open_application_area(self.interface(), handle, app_id)
    }

    /// Reads from the application area.
    ///
    /// Returns the number of bytes read.
    #[inline]
    pub fn get_application_area(
        &self,
        handle: &NfcDeviceHandle,
        buf: &mut [u8],
    ) -> Result<u32, DispatchError> {
        cmif::nfp::get_application_area(self.interface(), handle, buf)
    }

    /// Writes to the application area.
    #[inline]
    pub fn set_application_area(
        &self,
        handle: &NfcDeviceHandle,
        buf: &[u8],
    ) -> Result<(), DispatchError> {
        cmif::nfp::set_application_area(self.interface(), handle, buf)
    }

    /// Flushes pending writes to the amiibo.
    #[inline]
    pub fn flush(&self, handle: &NfcDeviceHandle) -> Result<(), DispatchError> {
        cmif::nfp::flush(self.interface(), handle)
    }

    /// Restores the amiibo to its last saved state.
    #[inline]
    pub fn restore(&self, handle: &NfcDeviceHandle) -> Result<(), DispatchError> {
        cmif::nfp::restore(self.interface(), handle)
    }

    /// Creates a new application area on the amiibo.
    #[inline]
    pub fn create_application_area(
        &self,
        handle: &NfcDeviceHandle,
        app_id: u32,
        buf: &[u8],
    ) -> Result<(), DispatchError> {
        cmif::nfp::create_application_area(self.interface(), handle, app_id, buf)
    }

    /// Recreates the application area on the amiibo. [3.0.0+]
    #[inline]
    pub fn recreate_application_area(
        &self,
        handle: &NfcDeviceHandle,
        app_id: u32,
        buf: &[u8],
    ) -> Result<(), DispatchError> {
        cmif::nfp::recreate_application_area(self.interface(), handle, app_id, buf)
    }

    /// Gets the application area size.
    #[inline]
    pub fn get_application_area_size(
        &self,
        handle: &NfcDeviceHandle,
    ) -> Result<u32, DispatchError> {
        cmif::nfp::get_application_area_size(self.interface(), handle)
    }

    /// Gets tag info for the detected tag.
    #[inline]
    pub fn get_tag_info(
        &self,
        handle: &NfcDeviceHandle,
        out: &mut NfpTagInfo,
    ) -> Result<(), DispatchError> {
        cmif::nfp::get_tag_info(self.interface(), handle, out)
    }

    /// Gets register info (requires Ram mount).
    #[inline]
    pub fn get_register_info(
        &self,
        handle: &NfcDeviceHandle,
        out: &mut NfpRegisterInfo,
    ) -> Result<(), DispatchError> {
        cmif::nfp::get_register_info(self.interface(), handle, out)
    }

    /// Gets common info (requires Ram mount).
    #[inline]
    pub fn get_common_info(
        &self,
        handle: &NfcDeviceHandle,
        out: &mut NfpCommonInfo,
    ) -> Result<(), DispatchError> {
        cmif::nfp::get_common_info(self.interface(), handle, out)
    }

    /// Gets model info (requires Rom mount).
    #[inline]
    pub fn get_model_info(
        &self,
        handle: &NfcDeviceHandle,
        out: &mut NfpModelInfo,
    ) -> Result<(), DispatchError> {
        cmif::nfp::get_model_info(self.interface(), handle, out)
    }

    /// Attaches to the activate event for a device (returns raw handle).
    #[inline]
    pub fn attach_activate_event(&self, handle: &NfcDeviceHandle) -> Result<u32, DispatchError> {
        cmif::nfp::attach_activate_event(self.interface(), handle)
    }

    /// Attaches to the deactivate event for a device (returns raw handle).
    #[inline]
    pub fn attach_deactivate_event(&self, handle: &NfcDeviceHandle) -> Result<u32, DispatchError> {
        cmif::nfp::attach_deactivate_event(self.interface(), handle)
    }

    /// Attaches to the availability change event (returns raw handle). [3.0.0+]
    #[inline]
    pub fn attach_availability_change_event(&self) -> Result<u32, DispatchError> {
        cmif::nfp::attach_availability_change_event(self.interface())
    }

    /// Gets the service state.
    #[inline]
    pub fn get_state(&self) -> Result<u32, DispatchError> {
        cmif::nfp::get_state(self.interface())
    }

    /// Gets the device state for a handle.
    #[inline]
    pub fn get_device_state(&self, handle: &NfcDeviceHandle) -> Result<u32, DispatchError> {
        cmif::nfp::get_device_state(self.interface(), handle)
    }

    /// Gets the NpadId for a device handle.
    #[inline]
    pub fn get_npad_id(&self, handle: &NfcDeviceHandle) -> Result<u32, DispatchError> {
        cmif::nfp::get_npad_id(self.interface(), handle)
    }

    /// Formats the amiibo tag (not available for User service type).
    #[inline]
    pub fn format(&self, handle: &NfcDeviceHandle) -> Result<(), DispatchError> {
        cmif::nfp::format(self.interface(), handle)
    }

    /// Gets admin info (not available for User service type).
    #[inline]
    pub fn get_admin_info(
        &self,
        handle: &NfcDeviceHandle,
        out: &mut NfpAdminInfo,
    ) -> Result<(), DispatchError> {
        cmif::nfp::get_admin_info(self.interface(), handle, out)
    }

    /// Gets register info private (not available for User service type).
    #[inline]
    pub fn get_register_info_private(
        &self,
        handle: &NfcDeviceHandle,
        out: &mut NfpRegisterInfoPrivate,
    ) -> Result<(), DispatchError> {
        cmif::nfp::get_register_info_private(self.interface(), handle, out)
    }

    /// Sets register info private (not available for User service type).
    #[inline]
    pub fn set_register_info_private(
        &self,
        handle: &NfcDeviceHandle,
        info: &NfpRegisterInfoPrivate,
    ) -> Result<(), DispatchError> {
        cmif::nfp::set_register_info_private(self.interface(), handle, info)
    }

    /// Deletes register info (not available for User service type).
    #[inline]
    pub fn delete_register_info(&self, handle: &NfcDeviceHandle) -> Result<(), DispatchError> {
        cmif::nfp::delete_register_info(self.interface(), handle)
    }

    /// Deletes the application area (not available for User service type).
    #[inline]
    pub fn delete_application_area(&self, handle: &NfcDeviceHandle) -> Result<(), DispatchError> {
        cmif::nfp::delete_application_area(self.interface(), handle)
    }

    /// Checks if an application area exists (not available for User service type).
    #[inline]
    pub fn exists_application_area(&self, handle: &NfcDeviceHandle) -> Result<bool, DispatchError> {
        cmif::nfp::exists_application_area(self.interface(), handle)
    }

    /// Gets all amiibo data (debug service type only).
    #[inline]
    pub fn get_all(
        &self,
        handle: &NfcDeviceHandle,
        out: &mut NfpData,
    ) -> Result<(), DispatchError> {
        cmif::nfp::get_all(self.interface(), handle, out)
    }

    /// Sets all amiibo data (debug service type only).
    #[inline]
    pub fn set_all(&self, handle: &NfcDeviceHandle, data: &NfpData) -> Result<(), DispatchError> {
        cmif::nfp::set_all(self.interface(), handle, data)
    }

    /// Flushes in debug mode (debug service type only).
    #[inline]
    pub fn flush_debug(&self, handle: &NfcDeviceHandle) -> Result<(), DispatchError> {
        cmif::nfp::flush_debug(self.interface(), handle)
    }

    /// Breaks the tag (debug service type only).
    #[inline]
    pub fn break_tag(
        &self,
        handle: &NfcDeviceHandle,
        break_type: NfpBreakType,
    ) -> Result<(), DispatchError> {
        cmif::nfp::break_tag(self.interface(), handle, break_type as u32)
    }

    /// Reads backup data (debug service type only).
    ///
    /// Returns the number of bytes read.
    #[inline]
    pub fn read_backup_data(
        &self,
        handle: &NfcDeviceHandle,
        buf: &mut [u8],
    ) -> Result<u32, DispatchError> {
        cmif::nfp::read_backup_data(self.interface(), handle, buf)
    }

    /// Writes backup data (debug service type only).
    #[inline]
    pub fn write_backup_data(
        &self,
        handle: &NfcDeviceHandle,
        buf: &[u8],
    ) -> Result<(), DispatchError> {
        cmif::nfp::write_backup_data(self.interface(), handle, buf)
    }

    /// Writes NTF data (debug service type only).
    #[inline]
    pub fn write_ntf(
        &self,
        handle: &NfcDeviceHandle,
        write_type: u32,
        buf: &[u8],
    ) -> Result<(), DispatchError> {
        cmif::nfp::write_ntf(self.interface(), handle, write_type, buf)
    }
}

impl Drop for NfpService {
    fn drop(&mut self) {
        let _ = cmif::nfp::finalize(self.interface());
    }
}

/// Connected NFC service wrapper.
///
/// The NFC service has two command ID layouts: pre-4.0.0 (legacy) and 4.0.0+.
/// Per IC-4, both are exposed as separate methods and the caller selects.
pub struct NfcService {
    domain: Domain,
    /// Object id of the interface sub-object inside `domain`.
    interface_id: u32,
    legacy: bool,
}

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for NfcService {}
unsafe impl Sync for NfcService {}

impl NfcService {
    /// Addresses the interface sub-object inside the service's domain.
    ///
    /// Built on demand rather than stored: a stored view would have to name a
    /// lifetime that borrows the `domain` field beside it, which a struct
    /// cannot express. The view closes nothing, so the sub-object outlives
    /// every call made through it and `Drop` finalizes it exactly once.
    #[inline]
    fn interface(&self) -> DomainObjectRef<'_> {
        // SAFETY: `interface_id` was returned by `create_interface` on this
        // same domain at connect time and is closed only in `Drop`, so it
        // names a live server-side object for as long as `self` exists.
        DomainObjectRef::from_raw_unchecked(self.domain.as_borrowed(), self.interface_id)
            .expect("interface object id is non-zero once stored")
    }

    /// Gets the service state (pre-4.0.0).
    #[inline]
    pub fn get_state_legacy(&self) -> Result<u32, DispatchError> {
        cmif::nfc::get_state_legacy(self.interface())
    }

    /// Checks if NFC is enabled (pre-4.0.0).
    #[inline]
    pub fn is_nfc_enabled_legacy(&self) -> Result<bool, DispatchError> {
        cmif::nfc::is_nfc_enabled_legacy(self.interface())
    }

    /// Gets the service state (4.0.0+).
    #[inline]
    pub fn get_state(&self) -> Result<u32, DispatchError> {
        cmif::nfc::get_state(self.interface())
    }

    /// Checks if NFC is enabled (4.0.0+).
    #[inline]
    pub fn is_nfc_enabled(&self) -> Result<bool, DispatchError> {
        cmif::nfc::is_nfc_enabled(self.interface())
    }

    /// Lists connected NFC devices (4.0.0+).
    #[inline]
    pub fn list_devices(&self, out: &mut [NfcDeviceHandle]) -> Result<i32, DispatchError> {
        cmif::nfc::list_devices(self.interface(), out)
    }

    /// Gets the device state for a handle (4.0.0+).
    #[inline]
    pub fn get_device_state(&self, handle: &NfcDeviceHandle) -> Result<u32, DispatchError> {
        cmif::nfc::get_device_state(self.interface(), handle)
    }

    /// Gets the NpadId for a device handle (4.0.0+).
    #[inline]
    pub fn get_npad_id(&self, handle: &NfcDeviceHandle) -> Result<u32, DispatchError> {
        cmif::nfc::get_npad_id(self.interface(), handle)
    }

    /// Starts NFC tag detection with protocol filter (4.0.0+).
    #[inline]
    pub fn start_detection(
        &self,
        handle: &NfcDeviceHandle,
        protocol: NfcProtocol,
    ) -> Result<(), DispatchError> {
        cmif::nfc::start_detection(self.interface(), handle, protocol.bits())
    }

    /// Stops NFC tag detection (4.0.0+).
    #[inline]
    pub fn stop_detection(&self, handle: &NfcDeviceHandle) -> Result<(), DispatchError> {
        cmif::nfc::stop_detection(self.interface(), handle)
    }

    /// Gets tag info for the detected tag (4.0.0+).
    #[inline]
    pub fn get_tag_info(
        &self,
        handle: &NfcDeviceHandle,
        out: &mut NfcTagInfo,
    ) -> Result<(), DispatchError> {
        cmif::nfc::get_tag_info(self.interface(), handle, out)
    }

    /// Attaches to the activate event (4.0.0+, returns raw handle).
    #[inline]
    pub fn attach_activate_event(&self, handle: &NfcDeviceHandle) -> Result<u32, DispatchError> {
        cmif::nfc::attach_activate_event(self.interface(), handle)
    }

    /// Attaches to the deactivate event (4.0.0+, returns raw handle).
    #[inline]
    pub fn attach_deactivate_event(&self, handle: &NfcDeviceHandle) -> Result<u32, DispatchError> {
        cmif::nfc::attach_deactivate_event(self.interface(), handle)
    }

    /// Attaches to the availability change event (4.0.0+, returns raw handle).
    #[inline]
    pub fn attach_availability_change_event(&self) -> Result<u32, DispatchError> {
        cmif::nfc::attach_availability_change_event(self.interface())
    }

    /// Reads Mifare blocks (4.0.0+).
    #[inline]
    pub fn read_mifare(
        &self,
        handle: &NfcDeviceHandle,
        out_block_data: &mut [NfcMifareReadBlockData],
        read_block_parameter: &[NfcMifareReadBlockParameter],
    ) -> Result<(), DispatchError> {
        cmif::nfc::read_mifare(
            self.interface(),
            handle,
            out_block_data,
            read_block_parameter,
        )
    }

    /// Writes Mifare blocks (4.0.0+).
    #[inline]
    pub fn write_mifare(
        &self,
        handle: &NfcDeviceHandle,
        write_block_parameter: &[NfcMifareWriteBlockParameter],
    ) -> Result<(), DispatchError> {
        cmif::nfc::write_mifare(self.interface(), handle, write_block_parameter)
    }

    /// Sends a raw command via pass-through (4.0.0+).
    ///
    /// Returns the number of bytes in the reply.
    #[inline]
    pub fn send_command_by_pass_through(
        &self,
        handle: &NfcDeviceHandle,
        timeout: u64,
        cmd_buf: &[u8],
        reply_buf: &mut [u8],
    ) -> Result<u32, DispatchError> {
        cmif::nfc::send_command_by_pass_through(
            self.interface(),
            handle,
            timeout,
            cmd_buf,
            reply_buf,
        )
    }

    /// Keeps the pass-through session alive (4.0.0+).
    #[inline]
    pub fn keep_pass_through_session(&self, handle: &NfcDeviceHandle) -> Result<(), DispatchError> {
        cmif::nfc::keep_pass_through_session(self.interface(), handle)
    }

    /// Releases the pass-through session (4.0.0+).
    #[inline]
    pub fn release_pass_through_session(
        &self,
        handle: &NfcDeviceHandle,
    ) -> Result<(), DispatchError> {
        cmif::nfc::release_pass_through_session(self.interface(), handle)
    }
}

impl Drop for NfcService {
    fn drop(&mut self) {
        let _ = if self.legacy {
            cmif::nfc::finalize_legacy(self.interface())
        } else {
            cmif::nfc::finalize(self.interface())
        };
    }
}

/// Connected NFC Mifare service wrapper.
pub struct NfcMifareService {
    domain: Domain,
    /// Object id of the interface sub-object inside `domain`.
    interface_id: u32,
}

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for NfcMifareService {}
unsafe impl Sync for NfcMifareService {}

impl NfcMifareService {
    /// Addresses the interface sub-object inside the service's domain.
    ///
    /// Built on demand rather than stored: a stored view would have to name a
    /// lifetime that borrows the `domain` field beside it, which a struct
    /// cannot express. The view closes nothing, so the sub-object outlives
    /// every call made through it and `Drop` finalizes it exactly once.
    #[inline]
    fn interface(&self) -> DomainObjectRef<'_> {
        // SAFETY: `interface_id` was returned by `create_interface` on this
        // same domain at connect time and is closed only in `Drop`, so it
        // names a live server-side object for as long as `self` exists.
        DomainObjectRef::from_raw_unchecked(self.domain.as_borrowed(), self.interface_id)
            .expect("interface object id is non-zero once stored")
    }

    /// Lists connected NFC devices.
    #[inline]
    pub fn list_devices(&self, out: &mut [NfcDeviceHandle]) -> Result<i32, DispatchError> {
        cmif::mifare::list_devices(self.interface(), out)
    }

    /// Starts NFC tag detection.
    #[inline]
    pub fn start_detection(&self, handle: &NfcDeviceHandle) -> Result<(), DispatchError> {
        cmif::mifare::start_detection(self.interface(), handle)
    }

    /// Stops NFC tag detection.
    #[inline]
    pub fn stop_detection(&self, handle: &NfcDeviceHandle) -> Result<(), DispatchError> {
        cmif::mifare::stop_detection(self.interface(), handle)
    }

    /// Reads Mifare blocks.
    #[inline]
    pub fn read_mifare(
        &self,
        handle: &NfcDeviceHandle,
        out_block_data: &mut [NfcMifareReadBlockData],
        read_block_parameter: &[NfcMifareReadBlockParameter],
    ) -> Result<(), DispatchError> {
        cmif::mifare::read_mifare(
            self.interface(),
            handle,
            out_block_data,
            read_block_parameter,
        )
    }

    /// Writes Mifare blocks.
    #[inline]
    pub fn write_mifare(
        &self,
        handle: &NfcDeviceHandle,
        write_block_parameter: &[NfcMifareWriteBlockParameter],
    ) -> Result<(), DispatchError> {
        cmif::mifare::write_mifare(self.interface(), handle, write_block_parameter)
    }

    /// Gets tag info.
    #[inline]
    pub fn get_tag_info(
        &self,
        handle: &NfcDeviceHandle,
        out: &mut NfcTagInfo,
    ) -> Result<(), DispatchError> {
        cmif::mifare::get_tag_info(self.interface(), handle, out)
    }

    /// Attaches to the activate event (returns raw handle).
    #[inline]
    pub fn attach_activate_event(&self, handle: &NfcDeviceHandle) -> Result<u32, DispatchError> {
        cmif::mifare::attach_activate_event(self.interface(), handle)
    }

    /// Attaches to the deactivate event (returns raw handle).
    #[inline]
    pub fn attach_deactivate_event(&self, handle: &NfcDeviceHandle) -> Result<u32, DispatchError> {
        cmif::mifare::attach_deactivate_event(self.interface(), handle)
    }

    /// Gets the service state.
    #[inline]
    pub fn get_state(&self) -> Result<u32, DispatchError> {
        cmif::mifare::get_state(self.interface())
    }

    /// Gets the device state.
    #[inline]
    pub fn get_device_state(&self, handle: &NfcDeviceHandle) -> Result<u32, DispatchError> {
        cmif::mifare::get_device_state(self.interface(), handle)
    }

    /// Gets the NpadId for a device handle.
    #[inline]
    pub fn get_npad_id(&self, handle: &NfcDeviceHandle) -> Result<u32, DispatchError> {
        cmif::mifare::get_npad_id(self.interface(), handle)
    }

    /// Attaches to the availability change event (returns raw handle).
    #[inline]
    pub fn attach_availability_change_event(&self) -> Result<u32, DispatchError> {
        cmif::mifare::attach_availability_change_event(self.interface())
    }
}

impl Drop for NfcMifareService {
    fn drop(&mut self) {
        let _ = cmif::mifare::finalize(self.interface());
    }
}

/// Connects to an NFP service (amiibo).
///
/// Performs SM lookup, converts to domain, creates the interface sub-object,
/// and initializes it with PID + ARUID + MCU version data.
pub fn connect_nfp_cmif(
    sm: &SmService,
    service_type: NfpServiceType,
    aruid: u64,
    version_data: &[NfcRequiredMcuVersionData],
) -> Result<NfpService, ConnectNfpCmifError> {
    let service_name = match service_type {
        NfpServiceType::User => proto::NFP_USER_SERVICE_NAME,
        NfpServiceType::Debug => proto::NFP_DBG_SERVICE_NAME,
        NfpServiceType::System => proto::NFP_SYS_SERVICE_NAME,
    };

    let handle = sm
        .get_service_handle_cmif(service_name)
        .map_err(ConnectNfpCmifError::GetService)?;

    let session = Session::open(handle);
    let domain = session
        .convert_to_domain()
        .map_err(|(_session, err)| ConnectNfpCmifError::ConvertToDomain(err))?;

    let raw_object_id = cmif::nfp::create_interface(domain.as_borrowed())
        .map_err(ConnectNfpCmifError::CreateInterface)?;

    // SAFETY: `raw_object_id` was just returned by `cmif::nfp::create_interface`
    // on this same domain, so it names a live server-side object inside it.
    let interface = DomainObjectRef::from_raw_unchecked(domain.as_borrowed(), raw_object_id)
        .ok_or(ConnectNfpCmifError::MissingInterface)?;

    cmif::nfp::initialize(interface, aruid, version_data)
        .map_err(ConnectNfpCmifError::Initialize)?;

    Ok(NfpService {
        domain,
        interface_id: raw_object_id,
    })
}

/// Errors returned by [`connect_nfp_cmif`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectNfpCmifError {
    #[error("failed to look up nfp service via sm")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
    #[error("failed to ConvertToDomain on nfp session")]
    ConvertToDomain(#[source] ConvertToDomainError),
    #[error("failed to create nfp interface sub-object")]
    CreateInterface(#[source] cmif::NfpCreateInterfaceError),
    #[error("CreateInterface response did not include the expected sub-object")]
    MissingInterface,
    #[error("failed to initialize nfp interface")]
    Initialize(#[source] DispatchError),
}

/// Connects to an NFC service.
///
/// The `initialize_cmd_id` determines which command layout to use:
/// - Pre-4.0.0: pass `true` for `legacy` to use legacy command IDs.
/// - 4.0.0+: pass `false` for `legacy` to use the new command IDs.
pub fn connect_nfc_cmif(
    sm: &SmService,
    service_type: NfcServiceType,
    aruid: u64,
    version_data: &[NfcRequiredMcuVersionData],
    legacy: bool,
) -> Result<NfcService, ConnectNfcCmifError> {
    let service_name = match service_type {
        NfcServiceType::User => proto::NFC_USER_SERVICE_NAME,
        NfcServiceType::System => proto::NFC_SYS_SERVICE_NAME,
    };

    let handle = sm
        .get_service_handle_cmif(service_name)
        .map_err(ConnectNfcCmifError::GetService)?;

    let session = Session::open(handle);
    let domain = session
        .convert_to_domain()
        .map_err(|(_session, err)| ConnectNfcCmifError::ConvertToDomain(err))?;

    let raw_object_id = cmif::nfc::create_interface(domain.as_borrowed())
        .map_err(ConnectNfcCmifError::CreateInterface)?;

    // SAFETY: `raw_object_id` was just returned by `cmif::nfc::create_interface`
    // on this same domain, so it names a live server-side object inside it.
    let interface = DomainObjectRef::from_raw_unchecked(domain.as_borrowed(), raw_object_id)
        .ok_or(ConnectNfcCmifError::MissingInterface)?;

    let init_result = if legacy {
        cmif::nfc::initialize_legacy(interface, aruid, version_data)
    } else {
        cmif::nfc::initialize(interface, aruid, version_data)
    };
    init_result.map_err(ConnectNfcCmifError::Initialize)?;

    Ok(NfcService {
        domain,
        interface_id: raw_object_id,
        legacy,
    })
}

/// Errors returned by [`connect_nfc_cmif`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectNfcCmifError {
    #[error("failed to look up nfc service via sm")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
    #[error("failed to ConvertToDomain on nfc session")]
    ConvertToDomain(#[source] ConvertToDomainError),
    #[error("failed to create nfc interface sub-object")]
    CreateInterface(#[source] cmif::NfcCreateInterfaceError),
    #[error("CreateInterface response did not include the expected sub-object")]
    MissingInterface,
    #[error("failed to initialize nfc interface")]
    Initialize(#[source] DispatchError),
}

/// Connects to the NFC Mifare service (`nfc:mf:u`).
pub fn connect_mifare_cmif(
    sm: &SmService,
    aruid: u64,
    version_data: &[NfcRequiredMcuVersionData],
) -> Result<NfcMifareService, ConnectMifareCmifError> {
    let handle = sm
        .get_service_handle_cmif(proto::NFC_MF_SERVICE_NAME)
        .map_err(ConnectMifareCmifError::GetService)?;

    let session = Session::open(handle);
    let domain = session
        .convert_to_domain()
        .map_err(|(_session, err)| ConnectMifareCmifError::ConvertToDomain(err))?;

    let raw_object_id = cmif::mifare::create_interface(domain.as_borrowed())
        .map_err(ConnectMifareCmifError::CreateInterface)?;

    // SAFETY: `raw_object_id` was just returned by `cmif::mifare::create_interface`
    // on this same domain, so it names a live server-side object inside it.
    let interface = DomainObjectRef::from_raw_unchecked(domain.as_borrowed(), raw_object_id)
        .ok_or(ConnectMifareCmifError::MissingInterface)?;

    cmif::mifare::initialize(interface, aruid, version_data)
        .map_err(ConnectMifareCmifError::Initialize)?;

    Ok(NfcMifareService {
        domain,
        interface_id: raw_object_id,
    })
}

/// Errors returned by [`connect_mifare_cmif`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectMifareCmifError {
    #[error("failed to look up nfc:mf:u service via sm")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
    #[error("failed to ConvertToDomain on nfc:mf:u session")]
    ConvertToDomain(#[source] ConvertToDomainError),
    #[error("failed to create mifare interface sub-object")]
    CreateInterface(#[source] cmif::MifareCreateInterfaceError),
    #[error("CreateInterface response did not include the expected sub-object")]
    MissingInterface,
    #[error("failed to initialize mifare interface")]
    Initialize(#[source] DispatchError),
}
