project('nx-svc', version : '0.1.0')

cargo = find_program('cargo', required : true)

## Static library
# Source files
inc = include_directories('include')

# Target
nx_svc_tgt = custom_target(
    'nx-svc',
    command : [
        cargo, 'build',
        '--package', meson.project_name(),
        '--profile', get_option('buildtype') == 'release' ? 'release' : 'dev',
        '--target-dir', meson.global_build_root() / 'cargo-target',
        '--artifact-dir', '@OUTDIR@',
        '--features', 'ffi',
    ],
    output : ['libnx_svc.a', 'libnx_svc.rlib'],
    console: true,
    build_by_default : true,
    build_always_stale: true,
)

#------------------------------------------------
# Dependency declaration
#------------------------------------------------
# Linker script for overriding libnx svc functions
nx_svc_ld_override = meson.current_source_dir() / 'svc_override.ld'

nx_svc_dep = declare_dependency(
    include_directories : inc,
    link_with : nx_svc_tgt[0],
)
