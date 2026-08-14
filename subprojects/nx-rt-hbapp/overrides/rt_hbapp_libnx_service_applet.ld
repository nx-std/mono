/*
 * Applet Service API — NRO-specific entry points only.
 * Rust: ffi/libnx/applet.rs (functions), ffi/libnx/env.rs (global variable)
 * libnx: services/applet.c
 *
 * Only the applet-type-sourcing symbols are aliased here; the homebrew NRO
 * reads its applet type at runtime from the loader configuration. The
 * kind-agnostic `applet*` accessor surface is owned by nx-rt-core's
 * rt_core_libnx_service_applet.ld.
 *
 * The __nx_applet_type alias below is also defined by nx-rt-nso's
 * rt_nso_libnx_service_applet.ld. This is not a collision: an NRO build links
 * nx-rt-hbapp and an NSO build links nx-rt-nso, never both — the two entry
 * crates are mutually exclusive at link time, so exactly one fragment owns
 * __nx_applet_type in any given final link.
 */
EXTERN(__nx_rt_hbapp__libnx_applet_type);
EXTERN(__nx_rt_hbapp__libnx_applet_initialize);
EXTERN(__nx_rt_hbapp__libnx_applet_get_applet_type);

__nx_applet_type = __nx_rt_hbapp__libnx_applet_type;
appletInitialize = __nx_rt_hbapp__libnx_applet_initialize;
appletGetAppletType = __nx_rt_hbapp__libnx_applet_get_applet_type;
