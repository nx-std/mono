/* Linker script — NSO applet bring-up overrides (always linked for an NSO build).
 *
 * Redirects the libnx C applet-type-sourcing entry points to nx-rt-nso Rust
 * FFI implementations. Every NSO carries a build-time Application Manager
 * identity (one of the six AppletType values), so this fragment is
 * unconditional — unlike the NRO's per-service fragments, which are gated by
 * use_nx_service_*.
 *
 * The kind-agnostic `applet*` accessor surface (appletExit,
 * appletGetOperationMode, …) is owned by nx-rt-core's
 * rt_core_libnx_service_applet.ld.
 *
 * The __nx_applet_type alias below is also defined by nx-rt-hbapp's
 * rt_hbapp_libnx_service_applet.ld. This is not a collision: an NRO build links
 * nx-rt-hbapp and an NSO build links nx-rt-nso, never both — the two entry
 * crates are mutually exclusive at link time, so exactly one fragment owns
 * __nx_applet_type in any given final link.
 *
 * Organization:
 *   - Rust FFI:  subprojects/nx-rt-nso/src/ffi/libnx/applet.rs
 *   - libnx C:   subprojects/libnx/src/nx/source/services/applet.c
 */

/*
 * Applet Service API — NSO-specific entry points only.
 * Rust: ffi/libnx/applet.rs
 * libnx: services/applet.c
 */
EXTERN(__nx_rt_nso__libnx_applet_type);
__nx_applet_type = __nx_rt_nso__libnx_applet_type;

EXTERN(__nx_rt_nso__libnx_applet_initialize);
EXTERN(__nx_rt_nso__libnx_applet_get_applet_type);

appletInitialize = __nx_rt_nso__libnx_applet_initialize;
appletGetAppletType = __nx_rt_nso__libnx_applet_get_applet_type;
