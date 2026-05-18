/* Linker script — NSO-process entry overrides (always linked for an NSO build).
 *
 * Redirects the libnx C runtime symbols specific to the NSO startup
 * (pm-handoff env setup, __argdata__ argv parsing) to nx-rt-nso Rust FFI
 * implementations. Kind-agnostic runtime + Service Manager symbols are owned
 * by nx-rt-core's rt_core_libnx_core.ld; the applet bring-up override lives in
 * the sibling rt_nso_libnx_service_applet.ld fragment.
 *
 * Organization:
 *   - Rust FFI:  subprojects/nx-rt-nso/src/ffi/libnx/*.rs
 *   - libnx C:   subprojects/libnx/src/nx/source/runtime/*.c
 */

/*
 * Environment / Loader Configuration
 * Rust: ffi/libnx/env.rs
 * libnx: runtime/env.c
 */
EXTERN(__nx_rt_nso__libnx_env_setup);
envSetup = __nx_rt_nso__libnx_env_setup;

/*
 * Argument Vector (argv)
 * Rust: ffi/libnx/argv.rs
 * libnx: runtime/argv.c
 */
EXTERN(__nx_rt_nso__libnx_argv_setup);
EXTERN(__nx_rt_nso__libnx_system_argc);
EXTERN(__nx_rt_nso__libnx_system_argv);

argvSetup = __nx_rt_nso__libnx_argv_setup;
__system_argc = __nx_rt_nso__libnx_system_argc;
__system_argv = __nx_rt_nso__libnx_system_argv;
