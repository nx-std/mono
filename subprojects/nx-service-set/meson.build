project('nx-service-set', version : '0.1.0')

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

# nx-sys-thread-tls
nx_sys_thread_tls_proj = subproject('nx-sys-thread-tls')
nx_sys_thread_tls_dep = nx_sys_thread_tls_proj.get_variable('nx_sys_thread_tls_dep')

# Dependencies list
deps = [
    nx_panic_handler_dep,
    nx_service_sm_dep,
    nx_sf_dep,
    nx_svc_dep,
    nx_sys_thread_tls_dep,
]

#---------------------------------------------------------------------------------
# Static library
#---------------------------------------------------------------------------------
# Target
nx_service_set_tgt = custom_target(
    'nx-service-set',
    command : [
        cargo, 'build',
        '--package', meson.project_name(),
        '--profile', get_option('buildtype') == 'release' ? 'release' : 'dev',
        '--target-dir', meson.global_build_root() / 'cargo-target',
        '--artifact-dir', '@OUTDIR@',
    ],
    output : ['libnx_service_set.rlib'],
    console : true,
    build_by_default : true,
    build_always_stale : true,
)

#---------------------------------------------------------------------------------
# Dependency declaration
#---------------------------------------------------------------------------------
nx_service_set_dep = declare_dependency(
    sources : nx_service_set_tgt,
    dependencies : deps,
)
