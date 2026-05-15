//! CMIF protocol operations for the HID System service.

mod button_config;
mod buttons;
mod custom_button_config;
mod system;
mod touch;
mod unique_pad;

pub(crate) use self::{
    button_config::*, buttons::*, custom_button_config::*, system::*, touch::*, unique_pad::*,
};

/// Error returned by event acquisition commands.
#[derive(Debug, thiserror::Error)]
pub enum AcquireEventError {
    #[error("failed to dispatch event acquisition")]
    Dispatch(#[source] nx_sf::service::DispatchError),
    #[error("event acquisition response did not include expected copy handle")]
    MissingHandle,
}
