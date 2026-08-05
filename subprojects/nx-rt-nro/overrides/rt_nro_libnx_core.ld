/* Linker script — homebrew-NRO entry overrides (always linked when use_nx_rt is enabled).
 *
 * Redirects the libnx C runtime symbols specific to the homebrew-NRO startup
 * (loader-config env setup, argv parsing, default service bring-up and
 * teardown, startup working directory, nxlink host) to nx-rt-nro Rust FFI
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
 * Default Service Bring-Up and Teardown
 * Rust: ffi/libnx/app.rs
 * libnx: runtime/init.c
 *
 * Both are weak upstream so a program can replace the whole sequence. A script
 * assignment beats any object definition, weak or strong, so claiming them
 * takes that away. What a program keeps is the `userAppInit` / `userAppExit`
 * hooks the Rust sequence still calls, which is what homebrew extending
 * startup actually uses; replacing the sequence outright is the system-module
 * pattern, and a system module is not an NRO.
 */
EXTERN(__nx_rt_nro__libnx_app_init);
EXTERN(__nx_rt_nro__libnx_app_exit);

__appInit = __nx_rt_nro__libnx_app_init;
__appExit = __nx_rt_nro__libnx_app_exit;

/*
 * Startup Working Directory
 * Rust: ffi/libnx/cwd.rs
 * libnx: runtime/devices/fs_dev.c
 *
 * Only this one symbol is claimed from fs_dev.c. The rest of that file is the
 * fsdev devoptab layer, which is a separate concern and stays with libnx.
 */
EXTERN(__nx_rt_nro__libnx_init_cwd);
__libnx_init_cwd = __nx_rt_nro__libnx_init_cwd;

/*
 * nxlink Host Address
 * Rust: ffi/libnx/env.rs
 * libnx: runtime/nxlink.c
 */
EXTERN(__nx_rt_nro__libnx_nxlink_host);
__nxlink_host = __nx_rt_nro__libnx_nxlink_host;
