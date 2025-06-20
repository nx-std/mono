project('nx-std-thread', version : '0.1.0')

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
nx_std_thread_tgt = custom_target(
    'nx-std-thread',
    command : [
        cargo, 'build',
        '--package', meson.project_name(),
        '--profile', get_option('buildtype') == 'release' ? 'release' : 'dev',
        '--target-dir', meson.global_build_root() / 'cargo-target',
        '--artifact-dir', '@OUTDIR@',
        '--features', 'ffi',
    ],
    output : ['libnx_std_thread.a', 'libnx_std_thread.rlib'],
    console: true,
    build_by_default : true,
    build_always_stale: true,
)

#------------------------------------------------
# Dependency declaration
#------------------------------------------------
# Linker script for overriding libnx thread functions
nx_std_thread_ld_override = meson.current_source_dir() / 'std_thread_override.ld'

nx_std_thread_dep = declare_dependency(
    include_directories : inc,
    link_with : nx_std_thread_tgt[0],
    dependencies : deps,
)
