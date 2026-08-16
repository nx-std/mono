project('nx-tls', version : '0.1.0')

cargo = find_program('cargo', required : true)

#---------------------------------------------------------------------------------
# Dependencies
#---------------------------------------------------------------------------------
# Rust dependencies here are just informative so Meson can build the dependencies in the correct order
nx_panic_handler_proj = subproject('nx-panic-handler')
nx_panic_handler_dep = nx_panic_handler_proj.get_variable('nx_panic_handler_dep')

# This crate sits above the runtime rather than below it, which is what lets it
# read the running firmware its version gates need and reach the process's
# service-manager session. `nx-rt-core` must never depend on this crate in
# return.
nx_rt_core_proj = subproject('nx-rt-core')
nx_rt_core_dep = nx_rt_core_proj.get_variable('nx_rt_core_dep')

nx_service_ssl_proj = subproject('nx-service-ssl')
nx_service_ssl_dep = nx_service_ssl_proj.get_variable('nx_service_ssl_dep')

nx_sf_proj = subproject('nx-sf')
nx_sf_dep = nx_sf_proj.get_variable('nx_sf_dep')

nx_sys_net_proj = subproject('nx-sys-net')
nx_sys_net_dep = nx_sys_net_proj.get_variable('nx_sys_net_dep')

deps = [
    nx_panic_handler_dep,
    nx_rt_core_dep,
    nx_service_ssl_dep,
    nx_sf_dep,
    nx_sys_net_dep,
]

#---------------------------------------------------------------------------------
# Static library
#---------------------------------------------------------------------------------
nx_tls_tgt = custom_target(
    'nx-tls',
    command : [
        cargo, 'build',
        '--package', meson.project_name(),
        '--profile', get_option('buildtype') == 'release' ? 'release' : 'dev',
        '--target-dir', meson.global_build_root() / 'cargo-target',
        '--artifact-dir', '@OUTDIR@',
    ],
    output : ['libnx_tls.rlib'],
    console : true,
    build_by_default : true,
    build_always_stale : true,
)

#---------------------------------------------------------------------------------
# Dependency declaration
#---------------------------------------------------------------------------------
# Linker script for overriding the libnx `ssl` client and its socket hand-offs
nx_tls_ld_override = meson.current_source_dir() / 'tls_override.ld'

nx_tls_dep = declare_dependency(
    sources : nx_tls_tgt,
    dependencies : deps,
)
