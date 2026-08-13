project('nx-rt-nso', version : '0.1.0')

cargo = find_program('cargo', required : true)

#---------------------------------------------------------------------------------
# Options
#---------------------------------------------------------------------------------
# Build-time Application Manager identity: the `nso_applet_type` combo reaches
# the crate as the rustc cfg value of the same name, which fixes the entry
# crate's `APPLET_TYPE` const. It is a cfg value rather than a Cargo feature
# because the six identities are mutually exclusive and Cargo features are
# additive: `cargo <cmd> --all-features` would select all six at once. `combo`
# resolves to a single choice, so the value is always unambiguous. The `none`
# choice selects the background-sysmodule profile: no Application Manager
# identity, the AM handshake skipped.
cargo_env = {
    'RUSTFLAGS' : '--cfg nso_applet_type="' + get_option('nso_applet_type') + '"',
}

cargo_features = []

# rt-link — the rustc-link pipeline's per-kind `pm`-launch `.crt0`.
# On the rustc pipeline this crate supplies its own `_start`; on the GCC
# pipeline `_start` comes from libnx's `switch_crt0.s`, so the feature stays
# off there to avoid a duplicate-`_start` link error.
if get_option('link_pipeline') == 'rustc'
    cargo_features += 'rt-link'
endif

#---------------------------------------------------------------------------------
# Dependencies
#---------------------------------------------------------------------------------
# Rust dependencies here are just informative so Meson can build the dependencies in the correct order
# nx-alloc (provides the #[global_allocator])
nx_alloc_proj = subproject('nx-alloc')
nx_alloc_dep = nx_alloc_proj.get_variable('nx_alloc_dep')

# nx-panic-handler
nx_panic_handler_proj = subproject('nx-panic-handler')
nx_panic_handler_dep = nx_panic_handler_proj.get_variable('nx_panic_handler_dep')

# nx-rt-core (the kind-agnostic runtime core)
nx_rt_core_proj = subproject('nx-rt-core')
nx_rt_core_dep = nx_rt_core_proj.get_variable('nx_rt_core_dep')

# nx-service-applet (the AppletType value + role taxonomy for applet-init)
nx_service_applet_proj = subproject('nx-service-applet')
nx_service_applet_dep = nx_service_applet_proj.get_variable('nx_service_applet_dep')

# nx-svc (SVC bindings — the __argdata__ memory query)
nx_svc_proj = subproject('nx-svc')
nx_svc_dep = nx_svc_proj.get_variable('nx_svc_dep')

# nx-sys-args (the command-line store the __argdata__ reader installs into)
nx_sys_args_proj = subproject('nx-sys-args')
nx_sys_args_dep = nx_sys_args_proj.get_variable('nx_sys_args_dep')

deps = [
    nx_alloc_dep,
    nx_panic_handler_dep,
    nx_rt_core_dep,
    nx_service_applet_dep,
    nx_svc_dep,
    nx_sys_args_dep,
]

# The main-thread registration step, which `nx-rt-core` owns and builds.
if get_option('use_nx_sys_thread').enabled()
    cargo_features += 'sys-thread'
endif

# The reservation-map step, which `nx-rt-core` owns and builds. Gated on
# `use_nx_sys_mem` rather than an option of its own: the heap-free `virtmem`
# substrate rides along with `nx-sys-mem`, as it does in `nx-std`.
if get_option('use_nx_sys_mem').enabled()
    cargo_features += 'sys-virtmem'
endif

# Linker-script override fragments. The NSO-core fragment (envSetup, argvSetup,
# __system_argc/argv) and the applet fragment (appletInitialize,
# appletGetAppletType, __nx_applet_type) are both always present — every NSO
# brings up env and argv, and every NSO carries an applet-type value. For a
# `none` background sysmodule that value is `None`: the applet fragment still
# binds __nx_applet_type, now to `None`, which is the signal that makes the
# runtime skip the Application Manager — no appletOE/appletAE proxy session is
# opened. The kind-agnostic libnx symbols are owned by nx-rt-core's
# rt_core_libnx_core.ld and rt_core_libnx_service_applet.ld (the latter binds
# the shared `applet*` accessor surface, incl. appletExit).
#
# The fragments are named `<crate>_<override-target>`: the `rt_nso_` prefix
# marks the owning crate, the `libnx` segment the override target (only
# `libnx` today).
ld_overrides = [
    meson.current_source_dir() / 'overrides' / 'rt_nso_libnx_core.ld',
    meson.current_source_dir() / 'overrides' / 'rt_nso_libnx_service_applet.ld',
]

#---------------------------------------------------------------------------------
# Static library
#---------------------------------------------------------------------------------
cargo_cmd = [
    cargo, 'build',
    '--package', meson.project_name(),
    '--profile', get_option('buildtype') == 'release' ? 'release' : 'dev',
    '--target-dir', meson.global_build_root() / 'cargo-target',
    '--artifact-dir', '@OUTDIR@',
    '--no-default-features',
]
if cargo_features.length() > 0
    cargo_cmd += ['--features', ','.join(cargo_features)]
endif

nx_rt_nso_tgt = custom_target(
    'nx-rt-nso',
    command : cargo_cmd,
    env : cargo_env,
    output : ['libnx_rt_nso.rlib'],
    console : true,
    build_by_default : true,
    build_always_stale : true,
)

#---------------------------------------------------------------------------------
# Dependency declaration
#---------------------------------------------------------------------------------
# Linker script fragments — both the NSO-core and applet fragments are always
# present. Consumers read this list via
# `nx_rt_nso_proj.get_variable('nx_rt_nso_ld_overrides')`.
nx_rt_nso_ld_overrides = ld_overrides

# Cargo environment carrying the `nso_applet_type` cfg. A consumer that compiles
# this crate through its own cargo invocation (`nx-std`, which links it into the
# umbrella staticlib) must pass the same environment, or its copy falls back to
# the `application` default in `.cargo/config.toml` and registers the process as
# an identity the build did not ask for. Read via
# `nx_rt_nso_proj.get_variable('nx_rt_nso_cargo_env')`.
nx_rt_nso_cargo_env = cargo_env

nx_rt_nso_dep = declare_dependency(
    sources : nx_rt_nso_tgt,
    dependencies : deps,
)
