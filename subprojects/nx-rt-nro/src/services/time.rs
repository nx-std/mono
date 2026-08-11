//! Time service state and singleton API.
//!
//! This module manages the Time service session and provides a singleton interface
//! for accessing time functionality throughout the application lifecycle.

use nx_service_time::{
    TimeService,
    TimeServiceType,
    TimeType,
};
use nx_std_sync::{
    once_lock::OnceLock,
    rwlock::RwLock,
};
use nx_time::tz::TzSpec;

use crate::services::sm;

/// Global Time state, lazily initialized.
static TIME_STATE: OnceLock<RwLock<Option<TimeState>>> = OnceLock::new();

/// Returns a reference to the Time state lock, initializing it if needed.
fn state() -> &'static RwLock<Option<TimeState>> {
    TIME_STATE.get_or_init(|| RwLock::new(None))
}

/// Opens the time service for this process.
///
/// Connects to the user clock, which is the one a program is entitled to read
/// without a system role.
///
/// Counts its callers: a second caller joins the session the first opened
/// rather than replacing it, and it closes when the last of them calls
/// [`exit`]. Without the count, two independent users of this service in one
/// process would each close it under the other.
///
/// # Panics
///
/// Panics if SM is not initialized.
pub fn init() -> Result<(), ConnectError> {
    {
        let mut guard = state().write();
        if let Some(ref mut time_state) = *guard {
            time_state.ref_count += 1;
            return Ok(());
        }
    }

    let sm_guard = sm::sm_session();
    let sm = sm_guard.as_ref().expect("SM not initialized");

    // Connect to Time service (time:u by default)
    let service = nx_service_time::connect(sm, TimeServiceType::User).map_err(ConnectError)?;

    let mut guard = state().write();
    *guard = Some(TimeState {
        service,
        ref_count: 1,
    });

    Ok(())
}

/// Anchors the realtime clock and publishes the device's timezone to the C environment.
///
/// This is libnx's `__libnx_init_time`. Horizon exposes no realtime clock a process can read
/// directly, so the wall clock is read once here, over IPC, and handed to [`nx_time::realtime`],
/// which derives every later reading from the counter-timer. The device's timezone rule is then
/// rendered as a POSIX `TZ` specification for newlib.
///
/// # Errors
///
/// Returns [`InitWallClockError`] if the Time service is not initialized or either query fails.
/// A failure at the timezone step still leaves the clock anchored.
// libnx picks the clock from its weak `__nx_time_type` global and retries with the default clock
// when a caller-selected one fails. That knob is a libnx extension carried by a libnx data
// symbol, which this port deliberately does not depend on: it always reads the user system
// clock, so there is nothing to fall back from.
pub fn init_wall_clock() -> Result<(), InitWallClockError> {
    let service = get_service().ok_or(InitWallClockError::NotInitialized)?;

    let unix_secs = service
        .get_current_time(TimeType::UserSystemClock)
        .map_err(InitWallClockError::GetCurrentTime)?;

    // Anchored before the timezone is resolved so that a timezone failure costs only the `TZ`
    // variable, not the clock — libnx sequences its two steps the same way.
    nx_time::realtime::set_anchor(unix_secs);

    let (_calendar, info) = service
        .to_calendar_time_with_my_rule(unix_secs)
        .map_err(InitWallClockError::ToCalendarTime)?;

    nx_time::tz::set(&TzSpec::new(&info.timezone_name, info.offset));

    Ok(())
}

/// Error returned by [`init_wall_clock`].
#[derive(Debug, thiserror::Error)]
pub enum InitWallClockError {
    /// The Time service has not been initialized.
    ///
    /// Occurs when the wall clock is anchored before [`init`] has opened the session, leaving no
    /// clock to read. The realtime clock is left unanchored.
    #[error("the Time service is not initialized")]
    NotInitialized,
    /// Failed to read the current time from the user system clock.
    ///
    /// Occurs when the shared-memory read or the IPC call to the user system clock fails. The
    /// realtime clock is left unanchored.
    #[error("failed to read the current time")]
    GetCurrentTime(#[source] nx_service_time::GetCurrentTimeError),
    /// Failed to resolve the device's timezone rule.
    ///
    /// Occurs when the timezone service cannot convert the timestamp to calendar time. The
    /// realtime clock is already anchored by this point, so only `TZ` is left unset.
    #[error("failed to resolve the device timezone")]
    ToCalendarTime(#[source] nx_service_time::ToCalendarTimeError),
}

/// Gets the Time service.
pub fn get_service() -> Option<impl core::ops::Deref<Target = TimeService> + 'static> {
    let guard = state().read();
    if guard.is_some() {
        Some(TimeServiceRef(guard))
    } else {
        None
    }
}

/// Exits the Time service.
pub fn exit() {
    let mut guard = state().write();
    if let Some(ref mut time_state) = *guard {
        time_state.ref_count = time_state.ref_count.saturating_sub(1);
        if time_state.ref_count == 0 {
            // `TimeService` is RAII; dropping the taken state closes the
            // kernel handle.
            let _ = guard.take();
        }
    }
}

/// Internal storage for Time service.
struct TimeState {
    /// Time service (IStaticService with clock and timezone services)
    service: TimeService,
    /// How many callers of [`init`] have not yet called [`exit`]
    ref_count: u32,
}

/// Wrapper for accessing TimeService through RwLockReadGuard.
struct TimeServiceRef(nx_std_sync::rwlock::RwLockReadGuard<'static, Option<TimeState>>);

impl core::ops::Deref for TimeServiceRef {
    type Target = TimeService;

    fn deref(&self) -> &Self::Target {
        // SAFETY: We only create TimeServiceRef when the option is Some
        &self.0.as_ref().unwrap().service
    }
}

/// Error returned by [`init`] when connecting to the Time service fails.
#[derive(Debug, thiserror::Error)]
#[error("failed to connect to Time service")]
pub struct ConnectError(#[source] pub nx_service_time::ConnectError);

#[cfg(feature = "ffi")]
impl nx_rt_core::error::ToResultCode for ConnectError {
    fn to_rc(self) -> nx_rt_core::error::ResultCode {
        use nx_sf::error::ToResultCode as _;

        self.0.to_rc()
    }
}
