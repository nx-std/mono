project('nx-rt', version : '0.1.0')

cargo = find_program('cargo', required : true)

#---------------------------------------------------------------------------------
# Dependencies
#---------------------------------------------------------------------------------
# Rust dependencies here are just informative so Meson can build the dependencies in the correct order
deps = []
cargo_features = []
ld_overrides = [meson.current_source_dir() / 'overrides' / 'rt_core.ld']

# nx-alloc (always required)
nx_alloc_proj = subproject('nx-alloc')
deps += nx_alloc_proj.get_variable('nx_alloc_dep')

# nx-panic-handler (always required)
nx_panic_handler_proj = subproject('nx-panic-handler')
deps += nx_panic_handler_proj.get_variable('nx_panic_handler_dep')

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

# nx-sys-sync (always required)
nx_sys_sync_proj = subproject('nx-sys-sync')
deps += nx_sys_sync_proj.get_variable('nx_sys_sync_dep')

# nx-sys-thread-tls (always required)
nx_sys_thread_tls_proj = subproject('nx-sys-thread-tls')
deps += nx_sys_thread_tls_proj.get_variable('nx_sys_thread_tls_dep')

# nx-service-apm
if get_option('use_nx_service_apm').enabled()
    nx_service_apm_proj = subproject('nx-service-apm')
    deps += nx_service_apm_proj.get_variable('nx_service_apm_dep')
    cargo_features += 'service-apm'
    ld_overrides += meson.current_source_dir() / 'overrides' / 'rt_service_apm.ld'
endif

# nx-service-applet
if get_option('use_nx_service_applet').enabled()
    nx_service_applet_proj = subproject('nx-service-applet')
    deps += nx_service_applet_proj.get_variable('nx_service_applet_dep')
    cargo_features += 'service-applet'
    ld_overrides += meson.current_source_dir() / 'overrides' / 'rt_service_applet.ld'
endif

# nx-service-hid (depends on applet at the Rust level)
if get_option('use_nx_service_hid').enabled()
    nx_service_hid_proj = subproject('nx-service-hid')
    deps += nx_service_hid_proj.get_variable('nx_service_hid_dep')
    cargo_features += 'service-hid'
    ld_overrides += meson.current_source_dir() / 'overrides' / 'rt_service_hid.ld'
endif

# nx-service-nv (depends on applet at the Rust level)
if get_option('use_nx_service_nv').enabled()
    nx_service_nv_proj = subproject('nx-service-nv')
    deps += nx_service_nv_proj.get_variable('nx_service_nv_dep')
    cargo_features += 'service-nv'
    ld_overrides += meson.current_source_dir() / 'overrides' / 'rt_service_nv.ld'
endif

# nx-service-set
if get_option('use_nx_service_set').enabled()
    nx_service_set_proj = subproject('nx-service-set')
    deps += nx_service_set_proj.get_variable('nx_service_set_dep')
    cargo_features += 'service-set'
    ld_overrides += meson.current_source_dir() / 'overrides' / 'rt_service_set.ld'
endif

# nx-service-time
if get_option('use_nx_service_time').enabled()
    nx_service_time_proj = subproject('nx-service-time')
    deps += nx_service_time_proj.get_variable('nx_service_time_dep')
    cargo_features += 'service-time'
    ld_overrides += meson.current_source_dir() / 'overrides' / 'rt_service_time.ld'
endif

# nx-service-vi
if get_option('use_nx_service_vi').enabled()
    nx_service_vi_proj = subproject('nx-service-vi')
    deps += nx_service_vi_proj.get_variable('nx_service_vi_dep')
    cargo_features += 'service-vi'
    ld_overrides += meson.current_source_dir() / 'overrides' / 'rt_service_vi.ld'
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
nx_rt_tgt = custom_target(
    'nx-rt',
    command : cargo_cmd,
    output : ['libnx_rt.rlib'],
    console : true,
    build_by_default : true,
    build_always_stale : true,
)

#---------------------------------------------------------------------------------
# Dependency declaration
#---------------------------------------------------------------------------------
# Linker script fragments — core is always present; per-service fragments are
# appended above when their `use_nx_service_*` option is enabled. Consumers
# read this list via `nx_rt_proj.get_variable('nx_rt_ld_overrides')`.
nx_rt_ld_overrides = ld_overrides

nx_rt_dep = declare_dependency(
    sources : nx_rt_tgt,
    dependencies : deps,
)
