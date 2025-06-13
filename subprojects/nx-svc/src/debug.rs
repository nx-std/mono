use super::raw;

/// Trigger a debug event
///
/// This function is used to trigger a debug event.
/// It will cause the system to break into the debugger.
///
/// # Arguments
/// * `reason` - The reason for the break event
pub fn break_event(reason: BreakReason, address: usize, size: usize) -> ! {
    let _ = unsafe { raw::__nx_svc_break(reason.into(), address, size) };
    unreachable!()
}

/// Break reasons for debug events
pub enum BreakReason {
    /// Panic
    Panic,
    /// Assert
    Assert,
    /// User
    User,
    /// PreLoadDll
    PreLoadDll,
    /// PostLoadDll
    PostLoadDll,
    /// PreUnloadDll
    PreUnloadDll,
    /// PostUnloadDll
    PostUnloadDll,
    /// CppException
    CppException,

    /// NotificationOnlyFlag
    NotificationOnlyFlag,
}

impl From<BreakReason> for raw::BreakReason {
    fn from(value: BreakReason) -> Self {
        match value {
            BreakReason::Panic => raw::BreakReason::Panic,
            BreakReason::Assert => raw::BreakReason::Assert,
            BreakReason::User => raw::BreakReason::User,
            BreakReason::PreLoadDll => raw::BreakReason::PreLoadDll,
            BreakReason::PostLoadDll => raw::BreakReason::PostLoadDll,
            BreakReason::PreUnloadDll => raw::BreakReason::PreUnloadDll,
            BreakReason::PostUnloadDll => raw::BreakReason::PostUnloadDll,
            BreakReason::CppException => raw::BreakReason::CppException,
            BreakReason::NotificationOnlyFlag => raw::BreakReason::NotificationOnlyFlag,
        }
    }
}
