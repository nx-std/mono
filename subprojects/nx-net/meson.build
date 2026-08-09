project('nx-net', version : '0.1.0')

cargo = find_program('cargo', required : true)

#---------------------------------------------------------------------------------
# Dependencies
#---------------------------------------------------------------------------------
# Rust dependencies here are just informative so Meson can build the dependencies in the correct order
nx_alloc_proj = subproject('nx-alloc')
nx_alloc_dep = nx_alloc_proj.get_variable('nx_alloc_dep')

nx_panic_handler_proj = subproject('nx-panic-handler')
nx_panic_handler_dep = nx_panic_handler_proj.get_variable('nx_panic_handler_dep')

nx_service_sfdnsres_proj = subproject('nx-service-sfdnsres')
nx_service_sfdnsres_dep = nx_service_sfdnsres_proj.get_variable('nx_service_sfdnsres_dep')

nx_service_sm_proj = subproject('nx-service-sm')
nx_service_sm_dep = nx_service_sm_proj.get_variable('nx_service_sm_dep')

nx_sf_proj = subproject('nx-sf')
nx_sf_dep = nx_sf_proj.get_variable('nx_sf_dep')

nx_std_sync_proj = subproject('nx-std-sync')
nx_std_sync_dep = nx_std_sync_proj.get_variable('nx_std_sync_dep')

nx_svc_proj = subproject('nx-svc')
nx_svc_dep = nx_svc_proj.get_variable('nx_svc_dep')

deps = [
    nx_alloc_dep,
    nx_panic_handler_dep,
    nx_service_sfdnsres_dep,
    nx_service_sm_dep,
    nx_sf_dep,
    nx_std_sync_dep,
    nx_svc_dep,
]

#---------------------------------------------------------------------------------
# Static library
#---------------------------------------------------------------------------------
nx_net_tgt = custom_target(
    'nx-net',
    command : [
        cargo, 'build',
        '--package', meson.project_name(),
        '--profile', get_option('buildtype') == 'release' ? 'release' : 'dev',
        '--target-dir', meson.global_build_root() / 'cargo-target',
        '--artifact-dir', '@OUTDIR@',
    ],
    output : ['libnx_net.rlib'],
    console : true,
    build_by_default : true,
    build_always_stale : true,
)

#---------------------------------------------------------------------------------
# Dependency declaration
#---------------------------------------------------------------------------------
# Linker script for overriding libnx resolver (getaddrinfo/netdb) functions
nx_net_ld_override = meson.current_source_dir() / 'net_override.ld'

nx_net_dep = declare_dependency(
    sources : nx_net_tgt,
    dependencies : deps,
)
