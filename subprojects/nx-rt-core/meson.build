project('nx-rt-core', version : '0.1.0')

cargo = find_program('cargo', required : true)

#---------------------------------------------------------------------------------
# Dependencies
#---------------------------------------------------------------------------------
# Rust dependencies here are just informative so Meson can build the dependencies in the correct order
# nx-alloc
nx_alloc_proj = subproject('nx-alloc')
nx_alloc_dep = nx_alloc_proj.get_variable('nx_alloc_dep')

# nx-panic-handler
nx_panic_handler_proj = subproject('nx-panic-handler')
nx_panic_handler_dep = nx_panic_handler_proj.get_variable('nx_panic_handler_dep')

# nx-service-sm (sm is the foundation for every other service)
nx_service_sm_proj = subproject('nx-service-sm')
nx_service_sm_dep = nx_service_sm_proj.get_variable('nx_service_sm_dep')

# nx-sf
nx_sf_proj = subproject('nx-sf')
nx_sf_dep = nx_sf_proj.get_variable('nx_sf_dep')

# nx-std-sync
nx_std_sync_proj = subproject('nx-std-sync')
nx_std_sync_dep = nx_std_sync_proj.get_variable('nx_std_sync_dep')

# nx-svc
nx_svc_proj = subproject('nx-svc')
nx_svc_dep = nx_svc_proj.get_variable('nx_svc_dep')

# nx-sys-sync
nx_sys_sync_proj = subproject('nx-sys-sync')
nx_sys_sync_dep = nx_sys_sync_proj.get_variable('nx_sys_sync_dep')

# nx-sys-thread-tls
nx_sys_thread_tls_proj = subproject('nx-sys-thread-tls')
nx_sys_thread_tls_dep = nx_sys_thread_tls_proj.get_variable('nx_sys_thread_tls_dep')

deps = [
    nx_alloc_dep,
    nx_panic_handler_dep,
    nx_service_sm_dep,
    nx_sf_dep,
    nx_std_sync_dep,
    nx_svc_dep,
    nx_sys_sync_dep,
    nx_sys_thread_tls_dep,
]

# The optional dependencies, each behind the option that selects the Cargo
# feature it backs, so Meson builds exactly what Cargo will compile.

# nx-service-applet (backs the `service-applet` feature's shared applet
# manager; activated transitively by the NRO/NSO entry crates)
if get_option('use_nx_service_applet').enabled()
    nx_service_applet_proj = subproject('nx-service-applet')
    nx_service_applet_dep = nx_service_applet_proj.get_variable('nx_service_applet_dep')
    deps += nx_service_applet_dep
endif

# nx-net (backs the `net` feature's name-resolution symbols)
if get_option('use_nx_net').enabled()
    nx_net_proj = subproject('nx-net')
    nx_net_dep = nx_net_proj.get_variable('nx_net_dep')
    deps += nx_net_dep
endif

# nx-service-bsd + nx-sys-net (back the `sys-net` feature's socket-driver
# bring-up, whose interface revision follows the running firmware)
if get_option('use_nx_sys_net').enabled()
    nx_service_bsd_proj = subproject('nx-service-bsd')
    nx_service_bsd_dep = nx_service_bsd_proj.get_variable('nx_service_bsd_dep')
    deps += nx_service_bsd_dep

    nx_sys_net_proj = subproject('nx-sys-net')
    nx_sys_net_dep = nx_sys_net_proj.get_variable('nx_sys_net_dep')
    deps += nx_sys_net_dep
endif

#---------------------------------------------------------------------------------
# Static library
#---------------------------------------------------------------------------------
nx_rt_core_tgt = custom_target(
    'nx-rt-core',
    command : [
        cargo, 'build',
        '--package', meson.project_name(),
        '--profile', get_option('buildtype') == 'release' ? 'release' : 'dev',
        '--target-dir', meson.global_build_root() / 'cargo-target',
        '--artifact-dir', '@OUTDIR@',
    ],
    output : ['libnx_rt_core.rlib'],
    console : true,
    build_by_default : true,
    build_always_stale : true,
)

#---------------------------------------------------------------------------------
# Dependency declaration
#---------------------------------------------------------------------------------
# Linker-script override fragments, named `<crate>_<override-target>`: the
# `rt_core_` prefix marks the owning crate, the `libnx` segment the override
# target (only `libnx` today). The kind-agnostic core fragment
# (`rt_core_libnx_core.ld` — libnx C runtime + Service Manager) is always
# present; the applet-accessor fragment (`rt_core_libnx_service_applet.ld`) is
# appended when `use_nx_service_applet` is enabled, because its
# `__nx_rt_core__libnx_applet_*` symbols exist only with the `service-applet`
# Cargo feature. Consumers read the list via
# `nx_rt_core_proj.get_variable('nx_rt_core_ld_overrides')`.
ld_overrides = [meson.current_source_dir() / 'overrides' / 'rt_core_libnx_core.ld']
if get_option('use_nx_service_applet').enabled()
    ld_overrides += meson.current_source_dir() / 'overrides' / 'rt_core_libnx_service_applet.ld'
endif
if get_option('use_nx_net').enabled()
    ld_overrides += meson.current_source_dir() / 'overrides' / 'rt_core_libnx_net.ld'
endif
if get_option('use_nx_sys_net').enabled()
    ld_overrides += meson.current_source_dir() / 'overrides' / 'rt_core_libnx_sys_net.ld'
endif
nx_rt_core_ld_overrides = ld_overrides

nx_rt_core_dep = declare_dependency(
    sources : nx_rt_core_tgt,
    dependencies : deps,
)
