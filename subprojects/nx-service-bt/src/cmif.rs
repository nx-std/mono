//! CMIF protocol operations for the Bluetooth user service.

use nx_sf::{
    cmif,
    hipc::{
        InPointer,
        OutPointer,
    },
    service::BorrowedSessionHandle,
};

use crate::{
    proto,
    types::{
        BtdrvBleEventType,
        BtdrvGattAttributeUuid,
        BtdrvGattId,
        NotificationIn,
        ReadCharacteristicIn,
        ReadDescriptorIn,
        SendIndicationIn,
        SetLeResponseIn,
        WriteCharacteristicIn,
        WriteDescriptorIn,
    },
};

fn dispatch_in_with_pid<T>(
    session: BorrowedSessionHandle<'_>,
    cmd_id: u32,
    value: &T,
) -> Result<(), DispatchError>
where
    T: zerocopy::IntoBytes + zerocopy::Immutable,
{
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(cmd_id)
        .with_data_value(value)
        .with_send_pid()
        .build();
    req.send(&mut buf, session)
        .map_err(DispatchError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(DispatchError::ParseResponse)?;

    Ok(())
}

fn dispatch_in_with_pid_and_pointer<T>(
    session: BorrowedSessionHandle<'_>,
    cmd_id: u32,
    value: &T,
    buffer: &[u8],
) -> Result<(), DispatchError>
where
    T: zerocopy::IntoBytes + zerocopy::Immutable,
{
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(cmd_id)
        .with_data_value(value)
        .with_send_pid()
        .add_in_pointer(InPointer::new(buffer))
        .build();
    req.send(&mut buf, session)
        .map_err(DispatchError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(DispatchError::ParseResponse)?;

    Ok(())
}

/// Error returned by BT dispatch operations.
#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// LeClientReadCharacteristic (cmd 0).
pub fn le_client_read_characteristic(
    session: BorrowedSessionHandle<'_>,
    connection_handle: u32,
    is_primary: bool,
    serv_id: &BtdrvGattId,
    char_id: &BtdrvGattId,
    auth_req: u8,
    applet_resource_user_id: u64,
) -> Result<(), DispatchError> {
    let input = ReadCharacteristicIn {
        is_primary: u8::from(is_primary),
        auth_req,
        pad: [0; 2],
        connection_handle,
        serv_id: *serv_id,
        char_id: *char_id,
        applet_resource_user_id,
    };

    dispatch_in_with_pid(session, proto::LE_CLIENT_READ_CHARACTERISTIC, &input)
}

/// LeClientReadDescriptor (cmd 1).
#[allow(clippy::too_many_arguments)]
pub fn le_client_read_descriptor(
    session: BorrowedSessionHandle<'_>,
    connection_handle: u32,
    is_primary: bool,
    serv_id: &BtdrvGattId,
    char_id: &BtdrvGattId,
    desc_id: &BtdrvGattId,
    auth_req: u8,
    applet_resource_user_id: u64,
) -> Result<(), DispatchError> {
    let input = ReadDescriptorIn {
        is_primary: u8::from(is_primary),
        auth_req,
        pad: [0; 2],
        connection_handle,
        serv_id: *serv_id,
        char_id: *char_id,
        desc_id: *desc_id,
        applet_resource_user_id,
    };

    dispatch_in_with_pid(session, proto::LE_CLIENT_READ_DESCRIPTOR, &input)
}

/// LeClientWriteCharacteristic (cmd 2).
#[allow(clippy::too_many_arguments)]
pub fn le_client_write_characteristic(
    session: BorrowedSessionHandle<'_>,
    connection_handle: u32,
    is_primary: bool,
    serv_id: &BtdrvGattId,
    char_id: &BtdrvGattId,
    buffer: &[u8],
    auth_req: u8,
    with_response: bool,
    applet_resource_user_id: u64,
) -> Result<(), DispatchError> {
    let input = WriteCharacteristicIn {
        is_primary: u8::from(is_primary),
        auth_req,
        with_response: u8::from(with_response),
        pad: 0,
        connection_handle,
        serv_id: *serv_id,
        char_id: *char_id,
        applet_resource_user_id,
    };

    dispatch_in_with_pid_and_pointer(
        session,
        proto::LE_CLIENT_WRITE_CHARACTERISTIC,
        &input,
        buffer,
    )
}

/// LeClientWriteDescriptor (cmd 3).
#[allow(clippy::too_many_arguments)]
pub fn le_client_write_descriptor(
    session: BorrowedSessionHandle<'_>,
    connection_handle: u32,
    is_primary: bool,
    serv_id: &BtdrvGattId,
    char_id: &BtdrvGattId,
    desc_id: &BtdrvGattId,
    buffer: &[u8],
    auth_req: u8,
    applet_resource_user_id: u64,
) -> Result<(), DispatchError> {
    let input = WriteDescriptorIn {
        is_primary: u8::from(is_primary),
        auth_req,
        pad: [0; 2],
        connection_handle,
        serv_id: *serv_id,
        char_id: *char_id,
        desc_id: *desc_id,
        applet_resource_user_id,
    };

    dispatch_in_with_pid_and_pointer(session, proto::LE_CLIENT_WRITE_DESCRIPTOR, &input, buffer)
}

/// LeClientRegisterNotification (cmd 4).
pub fn le_client_register_notification(
    session: BorrowedSessionHandle<'_>,
    connection_handle: u32,
    is_primary: bool,
    serv_id: &BtdrvGattId,
    char_id: &BtdrvGattId,
    applet_resource_user_id: u64,
) -> Result<(), DispatchError> {
    let input = NotificationIn {
        is_primary: u8::from(is_primary),
        pad: [0; 3],
        connection_handle,
        serv_id: *serv_id,
        char_id: *char_id,
        applet_resource_user_id,
    };

    dispatch_in_with_pid(session, proto::LE_CLIENT_REGISTER_NOTIFICATION, &input)
}

/// LeClientDeregisterNotification (cmd 5).
pub fn le_client_deregister_notification(
    session: BorrowedSessionHandle<'_>,
    connection_handle: u32,
    is_primary: bool,
    serv_id: &BtdrvGattId,
    char_id: &BtdrvGattId,
    applet_resource_user_id: u64,
) -> Result<(), DispatchError> {
    let input = NotificationIn {
        is_primary: u8::from(is_primary),
        pad: [0; 3],
        connection_handle,
        serv_id: *serv_id,
        char_id: *char_id,
        applet_resource_user_id,
    };

    dispatch_in_with_pid(session, proto::LE_CLIENT_DEREGISTER_NOTIFICATION, &input)
}

/// SetLeResponse (cmd 6).
pub fn set_le_response(
    session: BorrowedSessionHandle<'_>,
    server_if: u8,
    serv_uuid: &BtdrvGattAttributeUuid,
    char_uuid: &BtdrvGattAttributeUuid,
    buffer: &[u8],
    applet_resource_user_id: u64,
) -> Result<(), DispatchError> {
    let input = SetLeResponseIn {
        server_if,
        pad: [0; 3],
        serv_uuid: *serv_uuid,
        char_uuid: *char_uuid,
        pad2: [0; 4],
        applet_resource_user_id,
    };

    dispatch_in_with_pid_and_pointer(session, proto::SET_LE_RESPONSE, &input, buffer)
}

/// LeSendIndication (cmd 7).
#[allow(clippy::too_many_arguments)]
pub fn le_send_indication(
    session: BorrowedSessionHandle<'_>,
    server_if: u8,
    serv_uuid: &BtdrvGattAttributeUuid,
    char_uuid: &BtdrvGattAttributeUuid,
    buffer: &[u8],
    noconfirm: bool,
    applet_resource_user_id: u64,
) -> Result<(), DispatchError> {
    let input = SendIndicationIn {
        server_if,
        noconfirm: u8::from(noconfirm),
        pad: [0; 2],
        serv_uuid: *serv_uuid,
        char_uuid: *char_uuid,
        pad2: [0; 4],
        applet_resource_user_id,
    };

    dispatch_in_with_pid_and_pointer(session, proto::LE_SEND_INDICATION, &input, buffer)
}

/// GetLeEventInfo (cmd 8).
pub fn get_le_event_info(
    session: BorrowedSessionHandle<'_>,
    buffer: &mut [u8],
    applet_resource_user_id: u64,
) -> Result<BtdrvBleEventType, GetLeEventInfoError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(proto::GET_LE_EVENT_INFO)
        .with_data_value(&applet_resource_user_id)
        .with_send_pid()
        .add_out_pointer(OutPointer::new(buffer))
        .build();
    req.send(&mut buf, session)
        .map_err(GetLeEventInfoError::SendRequest)?;

    let resp = cmif::parse_response::<&u32>(&buf).map_err(GetLeEventInfoError::ParseResponse)?;

    let raw_type = *resp.payload;

    Ok(match raw_type {
        0 => BtdrvBleEventType::ClientRegistration,
        1 => BtdrvBleEventType::ServerRegistration,
        2 => BtdrvBleEventType::ConnectionUpdate,
        3 => BtdrvBleEventType::PreferredConnectionParameters,
        4 => BtdrvBleEventType::ClientConnection,
        5 => BtdrvBleEventType::ServerConnection,
        6 => BtdrvBleEventType::ScanResult,
        7 => BtdrvBleEventType::ScanFilter,
        8 => BtdrvBleEventType::ClientNotify,
        9 => BtdrvBleEventType::ClientCacheSave,
        10 => BtdrvBleEventType::ClientCacheLoad,
        11 => BtdrvBleEventType::ClientConfigureMtu,
        12 => BtdrvBleEventType::ServerAddAttribute,
        13 => BtdrvBleEventType::ServerAttributeOperation,
        other => return Err(GetLeEventInfoError::InvalidEventType(other)),
    })
}

/// Error returned by [`get_le_event_info`].
#[derive(Debug, thiserror::Error)]
pub enum GetLeEventInfoError {
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
    #[error("invalid BLE event type: {0}")]
    InvalidEventType(u32),
}

/// RegisterBleEvent (cmd 9).
pub fn register_ble_event(
    session: BorrowedSessionHandle<'_>,
    applet_resource_user_id: u64,
) -> Result<u32, RegisterBleEventError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(proto::REGISTER_BLE_EVENT)
        .with_data_value(&applet_resource_user_id)
        .with_send_pid()
        .build();
    req.send(&mut buf, session)
        .map_err(RegisterBleEventError::SendRequest)?;

    let resp = cmif::parse_response::<()>(&buf).map_err(RegisterBleEventError::ParseResponse)?;

    let Some(&raw_handle) = resp.copy_handles.first() else {
        return Err(RegisterBleEventError::MissingHandle);
    };

    Ok(raw_handle)
}

/// Error returned by [`register_ble_event`].
#[derive(Debug, thiserror::Error)]
pub enum RegisterBleEventError {
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
    #[error("missing event handle in response")]
    MissingHandle,
}
