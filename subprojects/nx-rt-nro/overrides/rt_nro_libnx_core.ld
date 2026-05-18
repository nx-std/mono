/* Linker script — homebrew-NRO entry overrides (always linked when use_nx_rt is enabled).
 *
 * Redirects the libnx C runtime symbols specific to the homebrew-NRO startup
 * (loader-config env setup, argv parsing, nxlink host) to nx-rt-nro Rust FFI
 * implementations. Kind-agnostic runtime + Service Manager symbols are owned
 * by nx-rt-core's rt_core_libnx_core.ld; per-service overrides live in sibling
 * rt_nro_libnx_service_*.ld fragments.
 *
 * Organization:
 *   - Rust FFI:  subprojects/nx-rt-nro/src/ffi/libnx/*.rs
 *   - libnx C:   subprojects/libnx/src/nx/source/runtime/*.c
 */

/*
 * Environment / Loader Configuration
 * Rust: ffi/libnx/env.rs
 * libnx: runtime/env.c
 */
EXTERN(__nx_rt_nro__libnx_env_setup);
envSetup = __nx_rt_nro__libnx_env_setup;

/*
 * Argument Vector (argv)
 * Rust: ffi/libnx/argv.rs
 * libnx: runtime/argv.c
 */
EXTERN(__nx_rt_nro__libnx_argv_setup);
EXTERN(__nx_rt_nro__libnx_system_argc);
EXTERN(__nx_rt_nro__libnx_system_argv);

argvSetup = __nx_rt_nro__libnx_argv_setup;
__system_argc = __nx_rt_nro__libnx_system_argc;
__system_argv = __nx_rt_nro__libnx_system_argv;

/*
 * nxlink Host Address
 * Rust: ffi/libnx/env.rs
 * libnx: runtime/nxlink.c
 */
EXTERN(__nx_rt_nro__libnx_nxlink_host);
__nxlink_host = __nx_rt_nro__libnx_nxlink_host;
