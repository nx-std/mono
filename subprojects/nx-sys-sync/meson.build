project('nx-sys-sync', version : '0.1.0')

cargo = find_program('cargo', required : true)

#------------------------------------------------
# Dependencies
#------------------------------------------------
# Rust dependencies here ar just informative so Meson can build the dependencies in the correct order
# nx-svc
nx_svc_proj = subproject('nx-svc')
nx_svc_dep = nx_svc_proj.get_variable('nx_svc_dep')

# nx-sys-thread
nx_sys_thread_proj = subproject('nx-sys-thread')
nx_sys_thread_dep = nx_sys_thread_proj.get_variable('nx_sys_thread_dep')

# Dependencies list
deps = [
    nx_svc_dep,
    nx_sys_thread_dep,
]

#------------------------------------------------
# Static library
#------------------------------------------------
# Include directories
inc = include_directories('include')

# Target
nx_sys_sync_tgt = custom_target(
    'nx-sys-sync',
    command : [
        cargo, 'build',
        '--package', meson.project_name(),
        '--profile', get_option('buildtype') == 'release' ? 'release' : 'dev',
        '--target-dir', meson.global_build_root() / 'cargo-target',
        '--artifact-dir', '@OUTDIR@',
        '--features', 'ffi',
    ],
    output : ['libnx_sys_sync.a', 'libnx_sys_sync.rlib'],
    console: true,
    build_by_default : true,
    build_always_stale: true,
)

#------------------------------------------------
# Dependency declaration
#------------------------------------------------
# Linker script for overriding libnx sync functions
nx_sys_sync_ld_override = meson.current_source_dir() / 'sys_sync_override.ld'

nx_sys_sync_dep = declare_dependency(
    include_directories : inc,
    link_with : nx_sys_sync_tgt[0],
    dependencies : deps,
)
