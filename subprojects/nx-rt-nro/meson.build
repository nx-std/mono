project('nx-rt-nro', version : '0.1.0')

cargo = find_program('cargo', required : true)

#---------------------------------------------------------------------------------
# Options
#---------------------------------------------------------------------------------
cargo_features = []

# rt-link — the rustc-link pipeline's per-kind hbloader `.crt0`.
# On the rustc pipeline this crate supplies its own `_start`; on the GCC
# pipeline `_start` comes from libnx's `switch_crt0.s`, so the feature stays
# off there to avoid a duplicate-`_start` link error.
if get_option('link_pipeline') == 'rustc'
    cargo_features += 'rt-link'
endif

#---------------------------------------------------------------------------------
# Dependencies
#---------------------------------------------------------------------------------
# Rust dependencies here are just informative so Meson can build the dependencies
# in the correct order. Only the always-required core of the crate is listed: the
# per-service dependencies exist behind Cargo features this build does not
# enable, and `nx-std` - the resolver that does enable them - subprojects each
# service crate itself when its feature is on.
deps = []

# nx-alloc (always required)
nx_alloc_proj = subproject('nx-alloc')
deps += nx_alloc_proj.get_variable('nx_alloc_dep')

# nx-panic-handler (always required)
nx_panic_handler_proj = subproject('nx-panic-handler')
deps += nx_panic_handler_proj.get_variable('nx_panic_handler_dep')

# nx-rt-core (always required — the kind-agnostic runtime core)
nx_rt_core_proj = subproject('nx-rt-core')
deps += nx_rt_core_proj.get_variable('nx_rt_core_dep')

# nx-service-sm (always required — sm is the foundation for every other service)
nx_service_sm_proj = subproject('nx-service-sm')
deps += nx_service_sm_proj.get_variable('nx_service_sm_dep')

# nx-sf (always required)
nx_sf_proj = subproject('nx-sf')
deps += nx_sf_proj.get_variable('nx_sf_dep')

# nx-svc (always required)
nx_svc_proj = subproject('nx-svc')
deps += nx_svc_proj.get_variable('nx_svc_dep')

# nx-std-sync (always required)
nx_std_sync_proj = subproject('nx-std-sync')
deps += nx_std_sync_proj.get_variable('nx_std_sync_dep')

# nx-sys-mem (always required)
nx_sys_mem_proj = subproject('nx-sys-mem')
deps += nx_sys_mem_proj.get_variable('nx_sys_mem_dep')

# nx-sys-thread-tls (always required)
nx_sys_thread_tls_proj = subproject('nx-sys-thread-tls')
deps += nx_sys_thread_tls_proj.get_variable('nx_sys_thread_tls_dep')

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

# Target
nx_rt_nro_tgt = custom_target(
    'nx-rt-nro',
    command : cargo_cmd,
    output : ['libnx_rt_nro.rlib'],
    console : true,
    build_by_default : true,
    build_always_stale : true,
)

#---------------------------------------------------------------------------------
# Dependency declaration
#---------------------------------------------------------------------------------
# Linker-script override fragments, named `<crate>_<override-target>`: the
# `rt_nro_` prefix marks the owning crate, the `libnx` segment the override
# target (only `libnx` today). The kind-agnostic `libnx` symbols are owned by
# `nx-rt-core`; this crate owns the NRO-specific entry points plus one fragment
# per service manager whose `<svc>Initialize` entry point lives here.
#
# Every fragment is exported unconditionally: this file decides nothing. Which
# fragments are linked - and which Cargo features compile the symbols they
# bind - is resolved by `nx-std`, the one place the `use_nx_*` feature set is
# resolved, so the alias linked and the initializer compiled cannot disagree.
overrides_dir = meson.current_source_dir() / 'overrides'

# The NRO-specific entry points (hbloader config parser, argv, applet dispatch)
nx_rt_nro_ld_override = overrides_dir / 'rt_nro_libnx_core.ld'
# One fragment per service manager
nx_rt_nro_service_apm_ld_override = overrides_dir / 'rt_nro_libnx_service_apm.ld'
nx_rt_nro_service_applet_ld_override = overrides_dir / 'rt_nro_libnx_service_applet.ld'
nx_rt_nro_service_applet_err_ld_override = overrides_dir / 'rt_nro_libnx_service_applet_err.ld'
nx_rt_nro_service_fs_ld_override = overrides_dir / 'rt_nro_libnx_service_fs.ld'
nx_rt_nro_service_hid_ld_override = overrides_dir / 'rt_nro_libnx_service_hid.ld'
nx_rt_nro_service_nv_ld_override = overrides_dir / 'rt_nro_libnx_service_nv.ld'
nx_rt_nro_service_set_ld_override = overrides_dir / 'rt_nro_libnx_service_set.ld'
nx_rt_nro_service_time_ld_override = overrides_dir / 'rt_nro_libnx_service_time.ld'
nx_rt_nro_service_vi_ld_override = overrides_dir / 'rt_nro_libnx_service_vi.ld'

nx_rt_nro_dep = declare_dependency(
    sources : nx_rt_nro_tgt,
    dependencies : deps,
)
