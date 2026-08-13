//! Screenshot upload (`caps:su`) service implementation.
//!
//! Provides access to the screenshot upload service for saving application
//! screenshots to the album.
//!
//! The service is connected once via [`connect_capssu_cmif`]; its methods are
//! then called directly, and the session is closed on drop.
//!
//! Callers choose which methods to call based on the target firmware version:
//! 4.0.0+ for the service itself, 7.0.0+ for
//! [`CapssuService::set_shim_library_version`] and
//! [`CapssuService::save_screen_shot_ex1`], 6.0.0+ for
//! [`CapssuService::save_screen_shot_ex2`].
//!
//! There is no wrapper that fills in the save parameters: a caller builds the
//! [`ScreenShotAttribute`](crate::ScreenShotAttribute),
//! [`ApplicationData`](crate::ApplicationData) or
//! [`UserIdList`](crate::UserIdList) it wants and calls the matching `Ex`
//! method.

use nx_service_sm::SmService;
use nx_sf::service::{
    BorrowedSessionHandle,
    Session,
};

mod cmif;
mod proto;

pub use self::{
    cmif::{
        SaveScreenShotEx0Error,
        SaveScreenShotEx1Error,
        SaveScreenShotEx2Error,
        SetShimVersionError,
    },
    proto::CAPSSU_SERVICE_NAME,
};
use crate::{
    album::ApplicationAlbumEntry,
    screenshot::{
        ApplicationData,
        ScreenShotAttribute,
    },
    user::UserIdList,
};

/// Screenshot upload service wrapper.
#[repr(transparent)]
pub struct CapssuService(Session);

impl CapssuService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// CMIF protocol methods.
impl CapssuService {
    /// Sets the shim library version. Should be called after connect on 7.0.0+.
    #[inline]
    pub fn set_shim_library_version(
        &self,
        version: u64,
        applet_resource_user_id: u64,
    ) -> Result<(), SetShimVersionError> {
        cmif::set_shim_library_version(self.0.handle(), version, applet_resource_user_id)
    }

    /// Saves a screenshot with the given attributes.
    ///
    /// `image` must be an RGBA8 1280x720 buffer (at least 0x384000 bytes).
    #[inline]
    pub fn save_screen_shot_ex0(
        &self,
        attr: &ScreenShotAttribute,
        report_option: u32,
        applet_resource_user_id: u64,
        image: &[u8],
    ) -> Result<ApplicationAlbumEntry, SaveScreenShotEx0Error> {
        cmif::save_screen_shot_ex0(
            self.0.handle(),
            attr,
            report_option,
            applet_resource_user_id,
            image,
        )
    }

    /// Saves a screenshot with attributes and application data. [7.0.0+]
    #[inline]
    pub fn save_screen_shot_ex1(
        &self,
        attr: &ScreenShotAttribute,
        report_option: u32,
        applet_resource_user_id: u64,
        appdata: &ApplicationData,
        image: &[u8],
    ) -> Result<ApplicationAlbumEntry, SaveScreenShotEx1Error> {
        cmif::save_screen_shot_ex1(
            self.0.handle(),
            attr,
            report_option,
            applet_resource_user_id,
            appdata,
            image,
        )
    }

    /// Saves a screenshot with attributes and user IDs. [6.0.0+]
    #[inline]
    pub fn save_screen_shot_ex2(
        &self,
        attr: &ScreenShotAttribute,
        report_option: u32,
        applet_resource_user_id: u64,
        list: &UserIdList,
        image: &[u8],
    ) -> Result<ApplicationAlbumEntry, SaveScreenShotEx2Error> {
        cmif::save_screen_shot_ex2(
            self.0.handle(),
            attr,
            report_option,
            applet_resource_user_id,
            list,
            image,
        )
    }
}

/// Connects to the screenshot upload service using CMIF.
pub fn connect_capssu_cmif(sm: &SmService) -> Result<CapssuService, ConnectCapssuCmifError> {
    let handle = sm
        .get_service_handle_cmif(CAPSSU_SERVICE_NAME)
        .map_err(ConnectCapssuCmifError)?;

    let service = Session::new(handle, 0);

    Ok(CapssuService(service))
}

/// Error returned by [`connect_capssu_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get caps:su service")]
pub struct ConnectCapssuCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
