//! Notification service wire-layout types.

use static_assertions::const_assert_eq;

/// Maximum alarms that can be registered at the same time by the host
/// application.
pub const MAX_ALARMS: usize = 8;

/// Weekly schedule alarm setting.
///
/// Each entry in `settings` encodes a day-of-week alarm time as a packed
/// `i16`: high byte = hour, low byte = minute. A value of `0xFFFF` (-1)
/// means the day is disabled.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct WeeklyScheduleAlarmSetting {
    pub _reserved: [u8; 0xa],
    pub settings: [i16; 7],
}

const_assert_eq!(size_of::<WeeklyScheduleAlarmSetting>(), 0x18);

/// Alarm setting stored and returned by the notification service.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct AlarmSetting {
    pub alarm_setting_id: u16,
    pub kind: u8,
    pub muted: u8,
    pub _pad: [u8; 4],
    pub uid: AccountUid,
    pub application_id: u64,
    pub _unk_x20: u64,
    pub schedule: WeeklyScheduleAlarmSetting,
}

const_assert_eq!(size_of::<AlarmSetting>(), 0x40);

/// Account user identifier (128-bit).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(C)]
pub struct AccountUid {
    pub uid: [u64; 2],
}

const_assert_eq!(size_of::<AccountUid>(), 0x10);

/// Decoded alarm time for a single day-of-week. Local time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlarmTime {
    pub hour: i32,
    pub minute: i32,
}

impl AlarmSetting {
    /// Creates a new alarm setting with all days disabled.
    pub fn new() -> Self {
        Self {
            alarm_setting_id: 0,
            kind: 0,
            muted: 0,
            _pad: [0; 4],
            uid: AccountUid::default(),
            application_id: 0,
            _unk_x20: 0,
            schedule: WeeklyScheduleAlarmSetting {
                _reserved: [0; 0xa],
                settings: [-1; 7],
            },
        }
    }

    /// Sets whether this alarm is muted (true = alarm turned off).
    #[inline]
    pub fn set_muted(&mut self, muted: bool) {
        self.muted = u8::from(muted);
    }

    /// Sets the account UID for this alarm.
    #[inline]
    pub fn set_uid(&mut self, uid: AccountUid) {
        self.uid = uid;
    }

    /// Returns whether the schedule setting for the given day is enabled.
    ///
    /// `day_of_week` must be 0–6 (Sun–Sat).
    pub fn is_day_enabled(&self, day_of_week: usize) -> Result<bool, DayOfWeekError> {
        if day_of_week >= 7 {
            return Err(DayOfWeekError);
        }
        let raw = self.schedule.settings[day_of_week];
        let hour = ((raw >> 8) & 0xFF) as u8;
        let minute = (raw & 0xFF) as u8;
        Ok(hour < 24 && minute < 60)
    }

    /// Gets the alarm time for the given day.
    ///
    /// `day_of_week` must be 0–6 (Sun–Sat). Returns `None` if the day is
    /// outside range.
    pub fn get_day_time(&self, day_of_week: usize) -> Result<AlarmTime, DayOfWeekError> {
        if day_of_week >= 7 {
            return Err(DayOfWeekError);
        }
        let raw = self.schedule.settings[day_of_week];
        Ok(AlarmTime {
            hour: ((raw >> 8) & 0xFF) as i32,
            minute: (raw & 0xFF) as i32,
        })
    }

    /// Enables the schedule setting for the given day with the specified time.
    ///
    /// `day_of_week` must be 0–6 (Sun–Sat). Uses local time.
    pub fn enable_day(
        &mut self,
        day_of_week: usize,
        hour: i32,
        minute: i32,
    ) -> Result<(), DayOfWeekError> {
        if day_of_week >= 7 {
            return Err(DayOfWeekError);
        }
        self.schedule.settings[day_of_week] =
            (((hour as u8) as i16) << 8) | ((minute as u8) as i16);
        Ok(())
    }

    /// Disables the schedule setting for the given day.
    ///
    /// `day_of_week` must be 0–6 (Sun–Sat).
    pub fn disable_day(&mut self, day_of_week: usize) -> Result<(), DayOfWeekError> {
        if day_of_week >= 7 {
            return Err(DayOfWeekError);
        }
        self.schedule.settings[day_of_week] = -1;
        Ok(())
    }
}

impl Default for AlarmSetting {
    fn default() -> Self {
        Self::new()
    }
}

/// Error returned when a day-of-week index is out of the valid 0–6 range.
#[derive(Debug, thiserror::Error)]
#[error("day_of_week must be 0-6 (Sun-Sat)")]
pub struct DayOfWeekError;
