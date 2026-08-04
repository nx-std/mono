//! IFactoryResetInterface CMIF commands.

use nx_sf::service::{
    DispatchError,
    Session,
};

use crate::{
    dispatch::dispatch_no_io,
    proto,
};

#[inline]
pub(crate) fn reset_to_factory_settings(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::FACTORY_RESET_TO_FACTORY_SETTINGS)
}

#[inline]
pub(crate) fn reset_to_factory_settings_without_user_save_data(
    service: &Session,
) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::FACTORY_RESET_WITHOUT_USER_SAVE_DATA)
}

#[inline]
pub(crate) fn reset_to_factory_settings_for_refurbishment(
    service: &Session,
) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::FACTORY_RESET_FOR_REFURBISHMENT)
}

#[inline]
pub(crate) fn reset_to_factory_settings_with_platform_region(
    service: &Session,
) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::FACTORY_RESET_WITH_PLATFORM_REGION)
}

#[inline]
pub(crate) fn reset_to_factory_settings_with_platform_region_authentication(
    service: &Session,
) -> Result<(), DispatchError> {
    dispatch_no_io(
        service,
        proto::FACTORY_RESET_WITH_PLATFORM_REGION_AUTHENTICATION,
    )
}
