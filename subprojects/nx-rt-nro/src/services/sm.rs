//! Service Manager (SM) — re-exported from [`nx_rt_core`].
//!
//! The Service Manager bootstrap is kind-agnostic: every output kind shares
//! one SM session and one override table. Its single authoritative
//! implementation lives in [`nx_rt_core::services::sm`]; this module
//! re-exports it so the per-service managers keep resolving
//! `crate::services::sm`.

// The C-facing lookup exists only when that boundary does.
#[cfg(feature = "ffi")]
pub use nx_rt_core::services::sm::get_service;
pub use nx_rt_core::services::sm::{
    ConnectError,
    DetachClientError,
    GetServiceError,
    InitializeError,
    MAX_OVERRIDES,
    NotInitializedError,
    RegisterServiceError,
    SmSession,
    TooManyOverridesError,
    UnregisterServiceError,
    add_override,
    detach_client,
    detach_client_cmif,
    detach_client_tipc,
    exit,
    get_override,
    get_service_handle,
    initialize,
    register_service,
    register_service_cmif,
    register_service_tipc,
    session,
    should_use_tipc,
    unregister_service,
    unregister_service_cmif,
    unregister_service_tipc,
};
