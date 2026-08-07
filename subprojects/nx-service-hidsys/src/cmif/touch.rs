//! Touch screen configuration commands.

use core::mem::size_of;

use nx_sf::service::{
    DispatchError,
    Session,
};
use zerocopy::IntoBytes as _;

use crate::{
    dispatch::dispatch_out,
    proto,
    types::{
        HidTouchScreenConfigurationForNx,
        IsFirmwareUpdateNeededIn,
        UniquePadId,
    },
};

/// GetTouchScreenDefaultConfiguration (cmd 1153, 9.0.0+).
pub(crate) fn get_touch_screen_default_configuration(
    service: &Session,
) -> Result<HidTouchScreenConfigurationForNx, DispatchError> {
    dispatch_out(service, proto::GET_TOUCH_SCREEN_DEFAULT_CONFIGURATION)
}

/// IsFirmwareUpdateNeededForNotification (cmd 1154, 9.0.0+). Sends PID.
pub(crate) fn is_firmware_update_needed_for_notification(
    service: &Session,
    unique_pad_id: UniquePadId,
    aruid: u64,
) -> Result<bool, DispatchError> {
    let input = IsFirmwareUpdateNeededIn {
        val: 1,
        pad: 0,
        unique_pad_id,
        applet_resource_user_id: aruid,
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::IS_FIRMWARE_UPDATE_NEEDED_FOR_NOTIFICATION)
        .in_raw(input.as_bytes())
        .send_pid()
        .out_size(size_of::<u8>())
        .send(&mut ipc_buf)?;

    Ok(*result.value::<u8>() & 1 != 0)
}
