project('nx-service-hid', version : '0.1.0')

cargo = find_program('cargo', required : true)

#---------------------------------------------------------------------------------
# Dependencies
#---------------------------------------------------------------------------------
# Rust dependencies here are just informative so Meson can build the dependencies in the correct order
# nx-panic-handler
nx_panic_handler_proj = subproject('nx-panic-handler')
nx_panic_handler_dep = nx_panic_handler_proj.get_variable('nx_panic_handler_dep')

# nx-service-sm
nx_service_sm_proj = subproject('nx-service-sm')
nx_service_sm_dep = nx_service_sm_proj.get_variable('nx_service_sm_dep')

# nx-sf
nx_sf_proj = subproject('nx-sf')
nx_sf_dep = nx_sf_proj.get_variable('nx_sf_dep')

# nx-svc
nx_svc_proj = subproject('nx-svc')
nx_svc_dep = nx_svc_proj.get_variable('nx_svc_dep')

# nx-sys-mem
nx_sys_mem_proj = subproject('nx-sys-mem')
nx_sys_mem_dep = nx_sys_mem_proj.get_variable('nx_sys_mem_dep')

# nx-std-sync
nx_std_sync_proj = subproject('nx-std-sync')
nx_std_sync_dep = nx_std_sync_proj.get_variable('nx_std_sync_dep')

# nx-sys-thread-tls
nx_sys_thread_tls_proj = subproject('nx-sys-thread-tls')
nx_sys_thread_tls_dep = nx_sys_thread_tls_proj.get_variable('nx_sys_thread_tls_dep')

# Dependencies list
deps = [
    nx_panic_handler_dep,
    nx_service_sm_dep,
    nx_sf_dep,
    nx_svc_dep,
    nx_sys_mem_dep,
    nx_std_sync_dep,
    nx_sys_thread_tls_dep,
]

#---------------------------------------------------------------------------------
# Static library
#---------------------------------------------------------------------------------
# Target
nx_service_hid_tgt = custom_target(
    'nx-service-hid',
    command : [
        cargo, 'build',
        '--package', meson.project_name(),
        '--profile', get_option('buildtype') == 'release' ? 'release' : 'dev',
        '--target-dir', meson.global_build_root() / 'cargo-target',
        '--artifact-dir', '@OUTDIR@',
    ],
    output : ['libnx_service_hid.rlib'],
    console : true,
    build_by_default : true,
    build_always_stale : true,
)

#---------------------------------------------------------------------------------
# Dependency declaration
#---------------------------------------------------------------------------------
nx_service_hid_dep = declare_dependency(
    sources : nx_service_hid_tgt,
    dependencies : deps,
)
