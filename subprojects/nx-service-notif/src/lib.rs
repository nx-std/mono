//! Alarm notification (`notif`) service implementation.
//!
//! Provides access to the alarm notification service for registering,
//! updating, listing, and deleting alarm settings on the Nintendo Switch.
//!
//! ## Architecture
//!
//! The service operates in domain mode. Two service variants exist:
//! - `notif:a` — Application variant, requires initialization (cmd 1000
//!   with PID) after domain conversion.
//! - `notif:s` — System variant, no post-initialization required.
//!
//! ## Divergence from libnx
//!
//! libnx's `notif.c` keeps a guarded global singleton with hosversion
//! checks (requires 9.0.0+) and includes convenience helpers for alarm
//! schedule manipulation and applet-based notification retrieval.
//!
//! This crate exposes paired [`connect_cmif_application`] /
//! [`connect_cmif_system`] functions instead of a service-type enum, and
//! places schedule helpers directly on [`AlarmSetting`]. The applet-based
//! functions (`notifGetNotificationSystemEvent`,
//! `notifTryPopNotifiedApplicationParameter`) are omitted — they wrap
//! applet APIs, not `notif` IPC.
//!
//! Per IC-4, this crate is hosversion-unaware — the 9.0.0+ requirement
//! is left to the caller.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::{ConvertToDomainError, DispatchError, Domain, Session};

mod cmif;
mod dispatch;
mod proto;
mod types;

pub use nx_sf::service::DispatchError as IpcDispatchError;

pub use self::{
    proto::{SERVICE_NAME_A, SERVICE_NAME_S},
    types::{
        AccountUid, AlarmSetting, AlarmTime, DayOfWeekError, MAX_ALARMS, WeeklyScheduleAlarmSetting,
    },
};

/// Connected notification service wrapper.
///
/// The service operates in domain mode with a single session.
pub struct NotifService {
    domain: Domain,
}

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for NotifService {}
unsafe impl Sync for NotifService {}

impl NotifService {
    /// Registers an alarm setting.
    ///
    /// `app_param` is an optional application parameter buffer (max 0x400
    /// bytes). Pass an empty slice if not needed. Returns the assigned
    /// alarm setting ID.
    #[inline]
    pub fn register_alarm_setting(
        &self,
        alarm_setting: &AlarmSetting,
        app_param: &[u8],
    ) -> Result<u16, DispatchError> {
        cmif::register_alarm_setting(&self.domain, alarm_setting, app_param)
    }

    /// Updates an existing alarm setting.
    ///
    /// `app_param` is an optional application parameter buffer (max 0x400
    /// bytes). Pass an empty slice if not needed.
    #[inline]
    pub fn update_alarm_setting(
        &self,
        alarm_setting: &AlarmSetting,
        app_param: &[u8],
    ) -> Result<(), DispatchError> {
        cmif::update_alarm_setting(&self.domain, alarm_setting, app_param)
    }

    /// Lists all registered alarm settings.
    ///
    /// Writes into the provided buffer and returns the number of entries
    /// written.
    #[inline]
    pub fn list_alarm_settings(&self, out: &mut [AlarmSetting]) -> Result<i32, DispatchError> {
        cmif::list_alarm_settings(&self.domain, out)
    }

    /// Loads the application parameter for the given alarm setting ID.
    ///
    /// Returns the actual number of bytes written to the output buffer.
    #[inline]
    pub fn load_application_parameter(
        &self,
        alarm_setting_id: u16,
        out: &mut [u8],
    ) -> Result<u32, DispatchError> {
        cmif::load_application_parameter(&self.domain, alarm_setting_id, out)
    }

    /// Deletes an alarm setting by ID.
    #[inline]
    pub fn delete_alarm_setting(&self, alarm_setting_id: u16) -> Result<(), DispatchError> {
        cmif::delete_alarm_setting(&self.domain, alarm_setting_id)
    }
}

/// Connects to the Application notification service (`notif:a`).
///
/// Converts the session to domain mode and sends the initialization
/// command (cmd 1000) with PID.
///
/// Only available on [9.0.0+] — hosversion gating is left to the caller
/// per IC-4.
pub fn connect_cmif_application(sm: &SmService) -> Result<NotifService, ConnectCmifError> {
    let domain = connect_domain(sm, proto::SERVICE_NAME_A)?;
    let service = NotifService { domain };

    if let Err(err) = cmif::initialize(&service.domain) {
        return Err(ConnectCmifError::Initialize(err));
    }

    Ok(service)
}

/// Connects to the System notification service (`notif:s`).
///
/// Converts the session to domain mode. No initialization command is sent.
///
/// Only available on [9.0.0+] — hosversion gating is left to the caller
/// per IC-4.
pub fn connect_cmif_system(sm: &SmService) -> Result<NotifService, ConnectCmifError> {
    let domain = connect_domain(sm, proto::SERVICE_NAME_S)?;
    Ok(NotifService { domain })
}

/// Connects to the given service name and converts the session to a domain.
fn connect_domain(sm: &SmService, name: nx_sf::ServiceName) -> Result<Domain, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(name)
        .map_err(ConnectCmifError::GetService)?;

    let session = Session::new(handle);

    session
        .convert_to_domain()
        .map_err(|(_session, err)| ConnectCmifError::ConvertToDomain(err))
}

/// Errors returned by [`connect_cmif_application`] and [`connect_cmif_system`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectCmifError {
    /// SM lookup for the service name failed.
    #[error("failed to look up notif service via sm")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
    /// Converting the session to a domain failed.
    #[error("failed to ConvertToDomain on notif session")]
    ConvertToDomain(#[source] ConvertToDomainError),
    /// Application initialization command (cmd 1000) failed.
    #[error("notif:a initialization command failed")]
    Initialize(#[source] DispatchError),
}
