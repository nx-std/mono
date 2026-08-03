//! Bluetooth user service (`bt`) implementation.
//!
//! Provides BLE GATT client/server operations for the Nintendo Switch.
//! Only available on \[5.0.0+\].
//!
//! ## Usage
//!
//! 1. Connect to the service via [`connect_cmif`].
//! 2. Call BLE GATT client/server methods on [`BtService`].
//! 3. Use [`BtService::register_ble_event`] to get an event for BLE notifications.
//! 4. Use [`BtService::get_le_event_info`] to read event details.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::{BorrowedSessionHandle, Session};

mod cmif;
mod proto;
mod types;

pub use self::{
    cmif::{DispatchError, GetLeEventInfoError, RegisterBleEventError},
    proto::SERVICE_NAME,
    types::{BtdrvBleEventType, BtdrvGattAttributeUuid, BtdrvGattId},
};

/// Bluetooth user service wrapper.
#[repr(transparent)]
pub struct BtService(Session);

impl BtService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }

    /// Reads a GATT characteristic value.
    #[inline]
    pub fn le_client_read_characteristic(
        &self,
        connection_handle: u32,
        is_primary: bool,
        serv_id: &BtdrvGattId,
        char_id: &BtdrvGattId,
        auth_req: u8,
        applet_resource_user_id: u64,
    ) -> Result<(), DispatchError> {
        cmif::le_client_read_characteristic(
            self.0.handle(),
            connection_handle,
            is_primary,
            serv_id,
            char_id,
            auth_req,
            applet_resource_user_id,
        )
    }

    /// Reads a GATT descriptor value.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn le_client_read_descriptor(
        &self,
        connection_handle: u32,
        is_primary: bool,
        serv_id: &BtdrvGattId,
        char_id: &BtdrvGattId,
        desc_id: &BtdrvGattId,
        auth_req: u8,
        applet_resource_user_id: u64,
    ) -> Result<(), DispatchError> {
        cmif::le_client_read_descriptor(
            self.0.handle(),
            connection_handle,
            is_primary,
            serv_id,
            char_id,
            desc_id,
            auth_req,
            applet_resource_user_id,
        )
    }

    /// Writes a GATT characteristic value.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn le_client_write_characteristic(
        &self,
        connection_handle: u32,
        is_primary: bool,
        serv_id: &BtdrvGattId,
        char_id: &BtdrvGattId,
        buffer: &[u8],
        auth_req: u8,
        with_response: bool,
        applet_resource_user_id: u64,
    ) -> Result<(), DispatchError> {
        cmif::le_client_write_characteristic(
            self.0.handle(),
            connection_handle,
            is_primary,
            serv_id,
            char_id,
            buffer,
            auth_req,
            with_response,
            applet_resource_user_id,
        )
    }

    /// Writes a GATT descriptor value.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn le_client_write_descriptor(
        &self,
        connection_handle: u32,
        is_primary: bool,
        serv_id: &BtdrvGattId,
        char_id: &BtdrvGattId,
        desc_id: &BtdrvGattId,
        buffer: &[u8],
        auth_req: u8,
        applet_resource_user_id: u64,
    ) -> Result<(), DispatchError> {
        cmif::le_client_write_descriptor(
            self.0.handle(),
            connection_handle,
            is_primary,
            serv_id,
            char_id,
            desc_id,
            buffer,
            auth_req,
            applet_resource_user_id,
        )
    }

    /// Registers for GATT characteristic notifications.
    #[inline]
    pub fn le_client_register_notification(
        &self,
        connection_handle: u32,
        is_primary: bool,
        serv_id: &BtdrvGattId,
        char_id: &BtdrvGattId,
        applet_resource_user_id: u64,
    ) -> Result<(), DispatchError> {
        cmif::le_client_register_notification(
            self.0.handle(),
            connection_handle,
            is_primary,
            serv_id,
            char_id,
            applet_resource_user_id,
        )
    }

    /// Deregisters from GATT characteristic notifications.
    #[inline]
    pub fn le_client_deregister_notification(
        &self,
        connection_handle: u32,
        is_primary: bool,
        serv_id: &BtdrvGattId,
        char_id: &BtdrvGattId,
        applet_resource_user_id: u64,
    ) -> Result<(), DispatchError> {
        cmif::le_client_deregister_notification(
            self.0.handle(),
            connection_handle,
            is_primary,
            serv_id,
            char_id,
            applet_resource_user_id,
        )
    }

    /// Sets a GATT server LE response.
    #[inline]
    pub fn set_le_response(
        &self,
        server_if: u8,
        serv_uuid: &BtdrvGattAttributeUuid,
        char_uuid: &BtdrvGattAttributeUuid,
        buffer: &[u8],
        applet_resource_user_id: u64,
    ) -> Result<(), DispatchError> {
        cmif::set_le_response(
            self.0.handle(),
            server_if,
            serv_uuid,
            char_uuid,
            buffer,
            applet_resource_user_id,
        )
    }

    /// Sends a GATT server LE indication or notification.
    #[inline]
    pub fn le_send_indication(
        &self,
        server_if: u8,
        serv_uuid: &BtdrvGattAttributeUuid,
        char_uuid: &BtdrvGattAttributeUuid,
        buffer: &[u8],
        noconfirm: bool,
        applet_resource_user_id: u64,
    ) -> Result<(), DispatchError> {
        cmif::le_send_indication(
            self.0.handle(),
            server_if,
            serv_uuid,
            char_uuid,
            buffer,
            noconfirm,
            applet_resource_user_id,
        )
    }

    /// Gets BLE event info, writing event data into the provided buffer.
    #[inline]
    pub fn get_le_event_info(
        &self,
        buffer: &mut [u8],
        applet_resource_user_id: u64,
    ) -> Result<BtdrvBleEventType, GetLeEventInfoError> {
        cmif::get_le_event_info(self.0.handle(), buffer, applet_resource_user_id)
    }

    /// Registers for BLE events, returning a copy handle for the event (autoclear=true).
    #[inline]
    pub fn register_ble_event(
        &self,
        applet_resource_user_id: u64,
    ) -> Result<u32, RegisterBleEventError> {
        cmif::register_ble_event(self.0.handle(), applet_resource_user_id)
    }
}

/// Connects to the Bluetooth user service (`bt`) using CMIF.
pub fn connect_cmif(sm: &SmService) -> Result<BtService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::new(handle, 0);

    Ok(BtService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get bt service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
