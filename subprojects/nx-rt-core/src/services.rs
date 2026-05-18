//! Kind-agnostic service managers.
//!
//! The Service Manager (`sm`) is the bootstrap every Horizon OS service
//! depends on, so it is shared by every output kind and lives in the
//! kind-agnostic core.
//!
//! The Application Manager (`applet`) manager is also shared: every NRO and
//! NSO process performs the same libnx-faithful applet handshake — only the
//! source of the [`AppletType`](nx_service_applet::AppletType) value differs
//! (an NRO reads it from the loader config at runtime; an NSO selects it at
//! build time). The manager is therefore re-homed here behind the
//! `service-applet` feature, leaving each entry crate only its sourcing of the
//! applet-type value. The remaining per-service managers gated behind their
//! `service-*` Cargo features stay with the entry crate that owns them.
//!
//! Each manager stores its session in module-local static state guarded by a
//! `RwLock`, exposing typed accessors that return RAII guards.

pub mod sm;

#[cfg(feature = "service-applet")]
pub mod applet;
