//! Time Service Implementation.
//!
//! This crate provides access to the Nintendo Switch's Time service, which handles:
//! - System time (user and network clocks)
//! - Steady clock (monotonic time)
//! - Calendar time conversion with timezone support
//! - Shared memory for fast time reads (6.0.0+)
//!
//! The Time service provides both IPC-based time queries and lock-free shared
//! memory reads for improved performance on firmware 6.0.0+.

#![no_std]

extern crate nx_panic_handler; // Provide #![panic_handler]

use core::ptr::NonNull;

use nx_service_sm::SmService;
use nx_sf::service::Service;
use nx_svc::ipc::Handle as SessionHandle;
use nx_sys_mem::shmem::{self as sys_shmem, Mapped, Permissions};

mod cmif;
mod proto;
pub mod shmem;
pub mod types;

pub use self::{
    cmif::{
        GetCurrentTimeError, GetSharedMemoryError, GetSteadyClockError, GetSystemClockError,
        GetTimeZoneServiceError, ToCalendarTimeError,
    },
    proto::{
        SERVICE_NAME_MENU, SERVICE_NAME_REPAIR, SERVICE_NAME_SYSTEM, SERVICE_NAME_SYSTEM_USER,
        SERVICE_NAME_USER,
    },
    types::{
        TimeCalendarAdditionalInfo, TimeCalendarTime, TimeServiceType,
        TimeStandardSteadyClockTimePointType, TimeSteadyClockTimePoint, TimeSystemClockContext,
        TimeType,
    },
};

/// Size of time service shared memory (6.0.0+).
const SHMEM_SIZE: usize = 0x1000;

/// Time service (IStaticService) session wrapper.
///
/// Provides access to system clocks, steady clock, and timezone operations.
pub struct TimeService {
    service: Service,
    user_system_clock: Service,
    network_system_clock: Option<Service>,
    steady_clock: Service,
    timezone_service: Service,
    shmem_ptr: Option<NonNull<u8>>,
    _shmem: Option<sys_shmem::SharedMemory<Mapped>>,
}

// SAFETY: TimeService is safe to send across threads because:
// - All Service instances are just session handles (u32)
// - shmem_ptr points to read-only shared memory that is thread-safe
// - _shmem manages a kernel shared memory handle which is thread-safe
unsafe impl Send for TimeService {}

// SAFETY: TimeService is safe to share across threads because:
// - All operations are thread-safe
// - Shared memory is read-only and designed for concurrent access
unsafe impl Sync for TimeService {}

impl TimeService {
    /// Returns the underlying service session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.service.session
    }

    /// Returns the user system clock session handle.
    #[inline]
    pub fn user_system_clock_session(&self) -> SessionHandle {
        self.user_system_clock.session
    }

    /// Returns the network system clock session handle, if available.
    #[inline]
    pub fn network_system_clock_session(&self) -> Option<SessionHandle> {
        self.network_system_clock.as_ref().map(|svc| svc.session)
    }

    /// Returns the steady clock session handle.
    #[inline]
    pub fn steady_clock_session(&self) -> SessionHandle {
        self.steady_clock.session
    }

    /// Returns the timezone service session handle.
    #[inline]
    pub fn timezone_service_session(&self) -> SessionHandle {
        self.timezone_service.session
    }

    /// Returns the shared memory pointer if available (6.0.0+).
    #[inline]
    pub fn shared_memory_ptr(&self) -> Option<*const u8> {
        self.shmem_ptr.map(|ptr| ptr.as_ptr() as *const u8)
    }

    /// Consumes and closes the time service session.
    pub fn close(self) {
        self.service.close();
        self.user_system_clock.close();
        if let Some(svc) = self.network_system_clock {
            svc.close();
        }
        self.steady_clock.close();
        self.timezone_service.close();
    }

    /// Gets the current time from the specified clock type.
    ///
    /// On firmware 6.0.0+, uses lock-free shared memory reads when available.
    /// Falls back to IPC calls on older firmware or if shared memory is unavailable.
    pub fn get_current_time(&self, clock_type: TimeType) -> Result<u64, GetCurrentTimeError> {
        // Try shared memory read first if available (6.0.0+)
        if let Some(shmem_ptr) = self.shmem_ptr {
            return self.get_current_time_from_shmem(shmem_ptr, clock_type);
        }

        // Fall back to IPC call
        let session = match clock_type {
            TimeType::UserSystemClock => self.user_system_clock.session,
            TimeType::NetworkSystemClock => self
                .network_system_clock
                .as_ref()
                .map(|svc| svc.session)
                .ok_or(GetCurrentTimeError::NetworkClockUnavailable)?,
            TimeType::LocalSystemClock => {
                // LocalSystemClock not supported in minimal scope
                return Err(GetCurrentTimeError::LocalClockNotSupported);
            }
        };

        cmif::get_current_time(session)
    }

    /// Gets current time from shared memory (6.0.0+).
    fn get_current_time_from_shmem(
        &self,
        shmem_ptr: NonNull<u8>,
        clock_type: TimeType,
    ) -> Result<u64, GetCurrentTimeError> {
        // SAFETY: shmem_ptr points to valid shared memory mapping
        unsafe {
            let steady = shmem::read_steady_clock(shmem_ptr.as_ptr());

            let context = match clock_type {
                TimeType::UserSystemClock => shmem::read_user_system_clock(shmem_ptr.as_ptr()),
                TimeType::NetworkSystemClock => {
                    shmem::read_network_system_clock(shmem_ptr.as_ptr())
                }
                TimeType::LocalSystemClock => {
                    return Err(GetCurrentTimeError::LocalClockNotSupported);
                }
            };

            // Verify source IDs match
            if context.timestamp.source_id != steady.source_id {
                return Err(GetCurrentTimeError::SourceIdMismatch);
            }

            // Compute current time: offset + steady_time
            let steady_time = Self::compute_steady_time(&steady);
            Ok((context.offset as u64).wrapping_add(steady_time))
        }
    }

    /// Computes the steady clock time from the time point context.
    fn compute_steady_time(context: &TimeStandardSteadyClockTimePointType) -> u64 {
        // Read current system tick counter
        let current_tick = unsafe { nx_cpu::control_regs::cntpct_el0() };

        // Convert ticks to nanoseconds
        // Formula: (tick * 1_000_000_000ns) / 19_200_000Hz = (tick * 625) / 12
        // This matches libnx's armTicksToNs() function
        let tick_ns = (current_tick * 625) / 12;

        // Add base time and convert to seconds
        ((context.base_time + tick_ns as i64) / 1_000_000_000) as u64
    }

    /// Converts a POSIX timestamp to calendar time using the device's timezone rule.
    #[inline]
    pub fn to_calendar_time_with_my_rule(
        &self,
        timestamp: u64,
    ) -> Result<(TimeCalendarTime, TimeCalendarAdditionalInfo), ToCalendarTimeError> {
        cmif::to_calendar_time_with_my_rule(self.timezone_service.session, timestamp)
    }
}

/// Connects to the time service.
///
/// # Arguments
///
/// * `sm` - Service manager session
/// * `service_type` - Which time service variant to connect to
///
/// # Returns
///
/// A connected [`TimeService`] instance on success.
pub fn connect(sm: &SmService, service_type: TimeServiceType) -> Result<TimeService, ConnectError> {
    // Get time service from service manager
    let service_name = match service_type {
        TimeServiceType::User => SERVICE_NAME_USER,
        TimeServiceType::Menu => SERVICE_NAME_MENU,
        TimeServiceType::System => SERVICE_NAME_SYSTEM,
        TimeServiceType::Repair => SERVICE_NAME_REPAIR,
        TimeServiceType::SystemUser => SERVICE_NAME_SYSTEM_USER,
    };

    let handle = sm
        .get_service_handle_cmif(service_name)
        .map_err(ConnectError::GetService)?;

    let service = Service {
        session: handle,
        own_handle: 1,
        object_id: 0,
        pointer_buffer_size: 0,
    };

    // Get user system clock (always required)
    let user_clock_handle = cmif::get_standard_user_system_clock(service.session)
        .map_err(ConnectError::GetUserSystemClock)?;

    let user_system_clock = Service {
        session: user_clock_handle,
        own_handle: 1,
        object_id: 0,
        pointer_buffer_size: 0,
    };

    // Get network system clock (best effort, may fail)
    let network_system_clock = cmif::get_standard_network_system_clock(service.session)
        .ok()
        .map(|handle| Service {
            session: handle,
            own_handle: 1,
            object_id: 0,
            pointer_buffer_size: 0,
        });

    // Get steady clock
    let steady_clock_handle =
        cmif::get_standard_steady_clock(service.session).map_err(ConnectError::GetSteadyClock)?;

    let steady_clock = Service {
        session: steady_clock_handle,
        own_handle: 1,
        object_id: 0,
        pointer_buffer_size: 0,
    };

    // Get timezone service
    let timezone_handle =
        cmif::get_time_zone_service(service.session).map_err(ConnectError::GetTimeZoneService)?;

    let timezone_service = Service {
        session: timezone_handle,
        own_handle: 1,
        object_id: 0,
        pointer_buffer_size: 0,
    };

    // Try to get shared memory (6.0.0+, best effort)
    let (shmem_ptr, _shmem) = match cmif::get_shared_memory_native_handle(service.session) {
        Ok(shmem_handle) => {
            // Map shared memory (0x1000 bytes, read-only)
            let shmem_unmapped = sys_shmem::load_remote(shmem_handle, SHMEM_SIZE, Permissions::R);

            match unsafe { sys_shmem::map(shmem_unmapped) } {
                Ok(shmem) => {
                    let ptr = NonNull::new(shmem.addr().unwrap() as *mut u8);
                    (ptr, Some(shmem))
                }
                Err(_) => (None, None),
            }
        }
        Err(_) => (None, None),
    };

    Ok(TimeService {
        service,
        user_system_clock,
        network_system_clock,
        steady_clock,
        timezone_service,
        shmem_ptr,
        _shmem,
    })
}

/// Error returned by [`connect`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    /// Failed to get service handle from SM.
    #[error("failed to get service")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
    /// Failed to get user system clock.
    #[error("failed to get user system clock")]
    GetUserSystemClock(#[source] GetSystemClockError),
    /// Failed to get steady clock.
    #[error("failed to get steady clock")]
    GetSteadyClock(#[source] GetSteadyClockError),
    /// Failed to get timezone service.
    #[error("failed to get timezone service")]
    GetTimeZoneService(#[source] GetTimeZoneServiceError),
}
