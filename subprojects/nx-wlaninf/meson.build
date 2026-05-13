project('nx-wlaninf', version : '0.1.0')

cargo = find_program('cargo', required : true)

#---------------------------------------------------------------------------------
# Dependencies
#---------------------------------------------------------------------------------
nx_panic_handler_proj = subproject('nx-panic-handler')
nx_panic_handler_dep = nx_panic_handler_proj.get_variable('nx_panic_handler_dep')

nx_service_sm_proj = subproject('nx-service-sm')
nx_service_sm_dep = nx_service_sm_proj.get_variable('nx_service_sm_dep')

nx_service_wlaninf_proj = subproject('nx-service-wlaninf')
nx_service_wlaninf_dep = nx_service_wlaninf_proj.get_variable('nx_service_wlaninf_dep')

nx_sf_proj = subproject('nx-sf')
nx_sf_dep = nx_sf_proj.get_variable('nx_sf_dep')

nx_svc_proj = subproject('nx-svc')
nx_svc_dep = nx_svc_proj.get_variable('nx_svc_dep')

deps = [
    nx_panic_handler_dep,
    nx_service_sm_dep,
    nx_service_wlaninf_dep,
    nx_sf_dep,
    nx_svc_dep,
]

#---------------------------------------------------------------------------------
# Static library
#---------------------------------------------------------------------------------
nx_wlaninf_tgt = custom_target(
    'nx-wlaninf',
    command : [
        cargo, 'build',
        '--package', meson.project_name(),
        '--profile', get_option('buildtype') == 'release' ? 'release' : 'dev',
        '--target-dir', meson.global_build_root() / 'cargo-target',
        '--artifact-dir', '@OUTDIR@',
    ],
    output : ['libnx_wlaninf.rlib'],
    console : true,
    build_by_default : true,
    build_always_stale : true,
)

#---------------------------------------------------------------------------------
# Dependency declaration
#---------------------------------------------------------------------------------
# Linker script overriding libnx's wlaninf.h ABI with the `__nx_wlaninf__*`
# Rust FFI. Consumed by nx-std via
# `nx_wlaninf_proj.get_variable('nx_wlaninf_ld_override')`.
nx_wlaninf_ld_override = meson.current_source_dir() / 'wlaninf_override.ld'

nx_wlaninf_dep = declare_dependency(
    sources : nx_wlaninf_tgt,
    dependencies : deps,
)
