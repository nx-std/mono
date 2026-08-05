//! The priority every request to `fsp-srv` is dispatched at.

/// How `fsp-srv` should schedule the requests this process sends it.
///
/// The choice is per process rather than per request: it is stored on the
/// service and travels as the context of each command sent afterwards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Priority {
    Normal = 0,
    Realtime = 1,
    Low = 2,
    Background = 3,
}
