//! The settings interface (`set`): the language, region and name a console is set up with.
//!
//! What this interface answers is what the console was configured with, and every program may ask
//! it. The system-wide settings a program may also *write* are the neighbouring [`set:sys`]
//! interface's, which is a separate session.
//!
//! [`set:sys`]: crate::set_sys

use nx_service_sm::SmService;
use nx_sf::service::{
    DispatchError,
    Session,
};

mod cmif;
mod proto;

pub use self::proto::{
    DeviceNickname,
    InvalidLanguageCode,
    Language,
    LanguageCode,
    LanguageCodeParseError,
    RegionCode,
    SERVICE_NAME,
    UnknownLanguage,
    UnknownRegionCode,
};

/// A connected session to the settings interface.
pub struct SetService(Session);

// SAFETY: every operation is a command on the session handle, and the kernel serializes
// `SendSyncRequest` per handle.
unsafe impl Send for SetService {}
unsafe impl Sync for SetService {}

impl SetService {
    /// Reads the tag of the language the console is set to.
    ///
    /// # Errors
    ///
    /// Returns [`GetLanguageCodeError::Dispatch`] when the command failed, and
    /// [`GetLanguageCodeError::InvalidCode`] when the console answered with something that is not
    /// a tag.
    pub fn get_language_code(&self) -> Result<LanguageCode, GetLanguageCodeError> {
        let raw = cmif::get_language_code(&self.0).map_err(GetLanguageCodeError::Dispatch)?;

        LanguageCode::try_from(raw).map_err(GetLanguageCodeError::InvalidCode)
    }

    /// Reads the tag for a language, including one the console does not offer.
    ///
    /// `[4.0.0+]`
    ///
    /// # Errors
    ///
    /// The same as [`get_language_code`](Self::get_language_code).
    pub fn get_language_code_for(
        &self,
        language: Language,
    ) -> Result<LanguageCode, GetLanguageCodeError> {
        let raw = cmif::language_code_for(&self.0, language.to_raw())
            .map_err(GetLanguageCodeError::Dispatch)?;

        LanguageCode::try_from(raw).map_err(GetLanguageCodeError::InvalidCode)
    }

    /// Reads how many language tags the console offers.
    ///
    /// `[4.0.0+]`
    ///
    /// Before that, use
    /// [`get_available_language_code_count_legacy`](Self::get_available_language_code_count_legacy).
    ///
    /// # Errors
    ///
    /// Returns [`DispatchError`] when the command failed. Nothing is read.
    pub fn get_available_language_code_count(&self) -> Result<i32, DispatchError> {
        cmif::get_available_language_code_count(&self.0, proto::GET_AVAILABLE_LANGUAGE_CODE_COUNT)
    }

    /// Reads how many language tags the console offers, on the interface before `[4.0.0]`.
    ///
    /// # Errors
    ///
    /// The same as [`get_available_language_code_count`](Self::get_available_language_code_count).
    pub fn get_available_language_code_count_legacy(&self) -> Result<i32, DispatchError> {
        cmif::get_available_language_code_count(
            &self.0,
            proto::GET_AVAILABLE_LANGUAGE_CODE_COUNT_LEGACY,
        )
    }

    /// Reads every language tag the console offers.
    ///
    /// `[4.0.0+]`
    ///
    /// Before that, use
    /// [`get_available_language_codes_legacy`](Self::get_available_language_codes_legacy).
    ///
    /// # Errors
    ///
    /// Returns [`GetAvailableLanguageCodesError::Dispatch`] when the command failed, and
    /// [`GetAvailableLanguageCodesError::InvalidCode`] when one of the tags read back is not a
    /// tag.
    pub fn get_available_language_codes(
        &self,
    ) -> Result<AvailableLanguageCodes, GetAvailableLanguageCodesError> {
        let mut raw = [0u64; AvailableLanguageCodes::MAX];
        let written = cmif::get_available_language_codes(&self.0, &mut raw)
            .map_err(GetAvailableLanguageCodesError::Dispatch)?;

        AvailableLanguageCodes::decode(&raw, written)
    }

    /// Reads every language tag the console offers, on the interface before `[4.0.0]`.
    ///
    /// That interface closes the session when asked for more tags than it has, so this reads the
    /// count first and asks for exactly that many.
    ///
    /// # Errors
    ///
    /// The same as [`get_available_language_codes`](Self::get_available_language_codes).
    pub fn get_available_language_codes_legacy(
        &self,
    ) -> Result<AvailableLanguageCodes, GetAvailableLanguageCodesError> {
        let count = self
            .get_available_language_code_count_legacy()
            .map_err(GetAvailableLanguageCodesError::Dispatch)?;

        let asked = AvailableLanguageCodes::in_range(count);
        let mut raw = [0u64; AvailableLanguageCodes::MAX];
        let written = cmif::get_available_language_codes_legacy(&self.0, &mut raw[..asked])
            .map_err(GetAvailableLanguageCodesError::Dispatch)?;

        AvailableLanguageCodes::decode(&raw, written)
    }

    /// Reads which region the console was sold into.
    ///
    /// # Errors
    ///
    /// Returns [`GetRegionCodeError::Dispatch`] when the command failed, and
    /// [`GetRegionCodeError::UnknownRegion`] when the console answered with a region this crate
    /// does not know.
    pub fn get_region_code(&self) -> Result<RegionCode, GetRegionCodeError> {
        let raw = cmif::get_region_code(&self.0).map_err(GetRegionCodeError::Dispatch)?;

        RegionCode::try_from(raw).map_err(GetRegionCodeError::UnknownRegion)
    }

    /// Reads whether the console is a retail demo unit.
    ///
    /// `[5.0.0+]`
    ///
    /// # Errors
    ///
    /// Returns [`DispatchError`] when the command failed. Nothing is read.
    pub fn get_quest_flag(&self) -> Result<bool, DispatchError> {
        cmif::get_quest_flag(&self.0).map(|flag| flag & 1 != 0)
    }

    /// Reads the name the owner gave the console.
    ///
    /// `[10.1.0+]`
    ///
    /// # Errors
    ///
    /// Returns [`DispatchError`] when the command failed. Nothing is read.
    pub fn get_device_nickname(&self) -> Result<DeviceNickname, DispatchError> {
        cmif::get_device_nickname(&self.0)
    }
}

/// Error returned by [`SetService::get_language_code`] and
/// [`SetService::get_language_code_for`].
#[derive(Debug, thiserror::Error)]
pub enum GetLanguageCodeError {
    /// The command failed
    ///
    /// Occurs when the request could not be sent, or the reply could not be decoded. Nothing was
    /// read.
    #[error("failed to read the language tag")]
    Dispatch(#[source] DispatchError),

    /// The answer is not a language tag
    ///
    /// Occurs when the field the console answered with is not NUL-padded ASCII. Nothing is
    /// decoded, because a tag this crate cannot read is one it cannot pass on either.
    #[error("the console answered with something that is not a language tag")]
    InvalidCode(#[source] InvalidLanguageCode),
}

#[cfg(feature = "ffi")]
impl nx_sf::error::ToResultCode for GetLanguageCodeError {
    fn to_rc(self) -> nx_sf::error::ResultCode {
        match self {
            Self::Dispatch(err) => err.to_rc(),
            Self::InvalidCode(_) => nx_sf::error::GENERIC_ERROR,
        }
    }
}

/// Every language tag a console offers.
///
/// The list is read into a field wide enough for the interface's own limit, so reading it costs
/// no allocation and a console that offers more tags than that is truncated rather than refused.
#[derive(Debug, Clone, Copy)]
pub struct AvailableLanguageCodes {
    /// The tags, of which only the first [`Self::len`] are answers.
    codes: [LanguageCode; AvailableLanguageCodes::MAX],
    /// How many tags the console offered.
    len: usize,
}

impl AvailableLanguageCodes {
    /// How many tags one console can offer.
    pub const MAX: usize = 0x40;

    /// Returns the tags the console offers.
    pub fn as_slice(&self) -> &[LanguageCode] {
        &self.codes[..self.len]
    }

    /// Returns how many of `count` tags fit in the field that carries them.
    ///
    /// A count is `i32` on the wire and neither bound is enforced there, so a negative count
    /// reads as none and a count past the field's width reads as a full field.
    fn in_range(count: i32) -> usize {
        usize::try_from(count).unwrap_or(0).min(Self::MAX)
    }

    /// Decodes the first `written` tags of `raw`.
    ///
    /// # Errors
    ///
    /// Returns [`GetAvailableLanguageCodesError::InvalidCode`] when one of them is not a tag.
    fn decode(
        raw: &[u64; Self::MAX],
        written: i32,
    ) -> Result<Self, GetAvailableLanguageCodesError> {
        let len = Self::in_range(written);
        let mut codes = [LanguageCode::EMPTY; Self::MAX];

        for (code, raw) in codes[..len].iter_mut().zip(&raw[..len]) {
            *code = LanguageCode::try_from(*raw)
                .map_err(GetAvailableLanguageCodesError::InvalidCode)?;
        }

        Ok(Self { codes, len })
    }
}

/// Error returned by [`SetService::get_available_language_codes`] and
/// [`SetService::get_available_language_codes_legacy`].
#[derive(Debug, thiserror::Error)]
pub enum GetAvailableLanguageCodesError {
    /// The command failed
    ///
    /// Occurs when the request could not be sent, or the reply could not be decoded. Nothing was
    /// read.
    #[error("failed to read the language tags the console offers")]
    Dispatch(#[source] DispatchError),

    /// One of the tags is not a language tag
    ///
    /// Occurs when a field in the list the console answered with is not NUL-padded ASCII. The
    /// whole list is refused rather than the entry skipped, because a list with a hole in it
    /// would renumber every tag after it.
    #[error("the console offered something that is not a language tag")]
    InvalidCode(#[source] InvalidLanguageCode),
}

#[cfg(feature = "ffi")]
impl nx_sf::error::ToResultCode for GetAvailableLanguageCodesError {
    fn to_rc(self) -> nx_sf::error::ResultCode {
        match self {
            Self::Dispatch(err) => err.to_rc(),
            Self::InvalidCode(_) => nx_sf::error::GENERIC_ERROR,
        }
    }
}

/// Error returned by [`SetService::get_region_code`].
#[derive(Debug, thiserror::Error)]
pub enum GetRegionCodeError {
    /// The command failed
    ///
    /// Occurs when the request could not be sent, or the reply could not be decoded. Nothing was
    /// read.
    #[error("failed to read the region the console was sold into")]
    Dispatch(#[source] DispatchError),

    /// The answer names no region this crate knows
    ///
    /// Occurs when the console answers with a region added after this crate's list was written.
    #[error("the console answered with a region this crate does not know")]
    UnknownRegion(#[source] UnknownRegionCode),
}

#[cfg(feature = "ffi")]
impl nx_sf::error::ToResultCode for GetRegionCodeError {
    fn to_rc(self) -> nx_sf::error::ResultCode {
        match self {
            Self::Dispatch(err) => err.to_rc(),
            Self::UnknownRegion(_) => nx_sf::error::GENERIC_ERROR,
        }
    }
}

/// Opens a session to the settings interface, asking the Service Manager over CMIF.
///
/// # Errors
///
/// Returns [`ConnectCmifError`] when the Service Manager refused to hand out the interface.
/// Nothing was opened.
pub fn connect_cmif(sm: &SmService) -> Result<SetService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    Ok(SetService(Session::new(handle, 0)))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get the set service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);

#[cfg(feature = "ffi")]
impl nx_sf::error::ToResultCode for ConnectCmifError {
    fn to_rc(self) -> nx_sf::error::ResultCode {
        self.0.to_rc()
    }
}

/// Opens a session to the settings interface, asking the Service Manager over TIPC.
///
/// The Service Manager speaks TIPC from `[12.0.0]`, and on Atmosphère. Which protocol *it* is
/// asked with does not change the session it hands back: the commands on that session are CMIF
/// either way.
///
/// # Errors
///
/// Returns [`ConnectTipcError`] when the Service Manager refused to hand out the interface.
/// Nothing was opened.
pub fn connect_tipc(sm: &SmService) -> Result<SetService, ConnectTipcError> {
    let handle = sm
        .get_service_handle_tipc(SERVICE_NAME)
        .map_err(ConnectTipcError)?;

    Ok(SetService(Session::new(handle, 0)))
}

/// Error returned by [`connect_tipc`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get the set service")]
pub struct ConnectTipcError(#[source] pub nx_service_sm::GetServiceTipcError);

#[cfg(feature = "ffi")]
impl nx_sf::error::ToResultCode for ConnectTipcError {
    fn to_rc(self) -> nx_sf::error::ResultCode {
        self.0.to_rc()
    }
}
