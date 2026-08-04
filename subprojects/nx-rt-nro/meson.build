project('nx-rt-nro', version : '0.1.0')

cargo = find_program('cargo', required : true)

#---------------------------------------------------------------------------------
# Dependencies
#---------------------------------------------------------------------------------
# Rust dependencies here are just informative so Meson can build the dependencies in the correct order
deps = []
cargo_features = []
# Linker-script override fragments, named `<crate>_<override-target>`: the
# `rt_nro_` prefix marks the owning crate, the `libnx` segment the override
# target (only `libnx` today). The kind-agnostic `libnx` symbols are owned by
# `nx-rt-core`'s `rt_core_libnx_core.ld`; this crate only overrides the
# NRO-specific entry points (`rt_nro_libnx_core.ld`) plus the per-service
# fragments appended below.
ld_overrides = [
    meson.current_source_dir() / 'overrides' / 'rt_nro_libnx_core.ld',
]

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

# nx-service-apm
if get_option('use_nx_service_apm').enabled()
    nx_service_apm_proj = subproject('nx-service-apm')
    deps += nx_service_apm_proj.get_variable('nx_service_apm_dep')
    cargo_features += 'service-apm'
    ld_overrides += meson.current_source_dir() / 'overrides' / 'rt_nro_libnx_service_apm.ld'
endif

# nx-service-applet
if get_option('use_nx_service_applet').enabled()
    nx_service_applet_proj = subproject('nx-service-applet')
    deps += nx_service_applet_proj.get_variable('nx_service_applet_dep')
    cargo_features += 'service-applet'
    ld_overrides += meson.current_source_dir() / 'overrides' / 'rt_nro_libnx_service_applet.ld'
endif

# nx-service-applet-err
if get_option('use_nx_service_applet_err').enabled()
    nx_service_applet_err_proj = subproject('nx-service-applet-err')
    deps += nx_service_applet_err_proj.get_variable('nx_service_applet_err_dep')
    cargo_features += 'service-applet-err'
    ld_overrides += meson.current_source_dir() / 'overrides' / 'rt_nro_libnx_service_applet_err.ld'
endif

# nx-service-fs
if get_option('use_nx_service_fs').enabled()
    nx_service_fs_proj = subproject('nx-service-fs')
    deps += nx_service_fs_proj.get_variable('nx_service_fs_dep')
    cargo_features += 'service-fs'
    ld_overrides += meson.current_source_dir() / 'overrides' / 'rt_nro_libnx_service_fs.ld'
endif

# nx-service-hid (depends on applet at the Rust level)
if get_option('use_nx_service_hid').enabled()
    nx_service_hid_proj = subproject('nx-service-hid')
    deps += nx_service_hid_proj.get_variable('nx_service_hid_dep')
    cargo_features += 'service-hid'
    ld_overrides += meson.current_source_dir() / 'overrides' / 'rt_nro_libnx_service_hid.ld'
endif

# nx-service-nv (depends on applet at the Rust level)
if get_option('use_nx_service_nv').enabled()
    nx_service_nv_proj = subproject('nx-service-nv')
    deps += nx_service_nv_proj.get_variable('nx_service_nv_dep')
    cargo_features += 'service-nv'
    ld_overrides += meson.current_source_dir() / 'overrides' / 'rt_nro_libnx_service_nv.ld'
endif

# nx-service-set
if get_option('use_nx_service_set').enabled()
    nx_service_set_proj = subproject('nx-service-set')
    deps += nx_service_set_proj.get_variable('nx_service_set_dep')
    cargo_features += 'service-set'
    ld_overrides += meson.current_source_dir() / 'overrides' / 'rt_nro_libnx_service_set.ld'
endif

# nx-service-time
#
# `nx-time` comes along with it: the startup wall-clock anchoring this manager performs
# (`__libnx_init_time`) installs its reading into `nx-time`, which owns the realtime clock the
# newlib `clock_gettime`/`gettimeofday` syscalls read back out.
if get_option('use_nx_service_time').enabled()
    nx_service_time_proj = subproject('nx-service-time')
    nx_time_proj = subproject('nx-time')
    deps += nx_service_time_proj.get_variable('nx_service_time_dep')
    deps += nx_time_proj.get_variable('nx_time_dep')
    cargo_features += 'service-time'
    ld_overrides += meson.current_source_dir() / 'overrides' / 'rt_nro_libnx_service_time.ld'
endif

# nx-service-vi
if get_option('use_nx_service_vi').enabled()
    nx_service_vi_proj = subproject('nx-service-vi')
    deps += nx_service_vi_proj.get_variable('nx_service_vi_dep')
    cargo_features += 'service-vi'
    ld_overrides += meson.current_source_dir() / 'overrides' / 'rt_nro_libnx_service_vi.ld'
endif

# rt-link — the rustc-link pipeline's per-kind hbloader `.crt0`.
# On the rustc pipeline this crate supplies its own `_start`; on the GCC
# pipeline `_start` comes from libnx's `switch_crt0.s`, so the feature stays
# off there to avoid a duplicate-`_start` link error.
if get_option('link_pipeline') == 'rustc'
    cargo_features += 'rt-link'
endif

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
# Linker script fragments — the NRO-core fragment is always present;
# per-service fragments are appended above when their `use_nx_service_*`
# option is enabled. Consumers read this list via
# `nx_rt_nro_proj.get_variable('nx_rt_nro_ld_overrides')`.
nx_rt_nro_ld_overrides = ld_overrides

nx_rt_nro_dep = declare_dependency(
    sources : nx_rt_nro_tgt,
    dependencies : deps,
)
