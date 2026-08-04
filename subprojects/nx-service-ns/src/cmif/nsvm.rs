//! ns:vm CMIF commands.

use nx_sf::service::{
    DispatchError,
    Session,
};

use crate::{
    dispatch::dispatch_out,
    proto,
    types::NcmContentMetaKey,
};

/// NeedsUpdateVulnerability (cmd 1200).
#[inline]
pub(crate) fn needs_update_vulnerability(service: &Session) -> Result<bool, DispatchError> {
    let val: u8 = dispatch_out(service, proto::NSVM_NEEDS_UPDATE_VULNERABILITY)?;
    Ok(val & 1 != 0)
}

/// GetSafeSystemVersion (cmd 1202).
#[inline]
pub(crate) fn get_safe_system_version(
    service: &Session,
) -> Result<NcmContentMetaKey, DispatchError> {
    dispatch_out(service, proto::NSVM_GET_SAFE_SYSTEM_VERSION)
}
