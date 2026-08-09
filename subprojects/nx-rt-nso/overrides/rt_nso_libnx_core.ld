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
 * Runtime Startup and Exit
 * Rust: ffi/libnx/init.rs
 * libnx: runtime/init.c
 *
 * The outermost pair: `__libnx_init` is what the `.crt0` branches to once the
 * image has relocated itself, and `__libnx_exit` is what newlib's
 * `__syscall_exit` reaches on the way out. That one is referenced weakly
 * there — `if (&__libnx_exit)` — so defining it is what keeps `exit()` from
 * falling into the `for (;;)` behind the test.
 *
 * The same pair is aliased by nx-rt-nro's rt_nro_libnx_core.ld. The two never
 * collide: the `nx_rt_kind` option selects one entry crate, so exactly one of
 * the fragments reaches any given final link.
 *
 * `__appInit` and `__appExit` are deliberately absent: this output kind has
 * not taken over the default-service sequence, so the Rust startup order calls
 * libnx's own by name and the per-service fragments redirect inside it.
 */
EXTERN(__nx_rt_nso__libnx_init);
EXTERN(__nx_rt_nso__libnx_exit);

__libnx_init = __nx_rt_nso__libnx_init;
__libnx_exit = __nx_rt_nso__libnx_exit;

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
