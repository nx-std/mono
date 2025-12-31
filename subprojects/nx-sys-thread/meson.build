project('nx-sys-thread', version : '0.1.0')

cargo = find_program('cargo', required : true)

#---------------------------------------------------------------------------------
# Dependencies
#---------------------------------------------------------------------------------
# Rust dependencies here are just informative so Meson can build the dependencies in the correct order
# nx-panic-handler
nx_panic_handler_proj = subproject('nx-panic-handler')
nx_panic_handler_dep = nx_panic_handler_proj.get_variable('nx_panic_handler_dep')

# nx-cpu
nx_cpu_proj = subproject('nx-cpu')
nx_cpu_dep = nx_cpu_proj.get_variable('nx_cpu_dep')

# nx-std-sync
nx_std_sync_proj = subproject('nx-std-sync')
nx_std_sync_dep = nx_std_sync_proj.get_variable('nx_std_sync_dep')

# Dependencies list
deps = [
    nx_panic_handler_dep,
    nx_cpu_dep,
    nx_std_sync_dep,
]

#---------------------------------------------------------------------------------
# Static library
#---------------------------------------------------------------------------------
# Include directories
inc = include_directories('include')

# Target
nx_sys_thread_tgt = custom_target(
    'nx-sys-thread',
    command : [
        cargo, 'build',
        '--package', meson.project_name(),
        '--profile', get_option('buildtype') == 'release' ? 'release' : 'dev',
        '--target-dir', meson.global_build_root() / 'cargo-target',
        '--artifact-dir', '@OUTDIR@',
        '--features', 'ffi',
    ],
    output : ['libnx_sys_thread.a', 'libnx_sys_thread.rlib'],
    console : true,
    build_by_default : true,
    build_always_stale : true,
)

#---------------------------------------------------------------------------------
# Dependency declaration
#---------------------------------------------------------------------------------
# Linker script for overriding libnx sys-thread functions
nx_sys_thread_ld_override = meson.current_source_dir() / 'sys_thread_override.ld'

nx_sys_thread_dep = declare_dependency(
    include_directories : inc,
    link_with : nx_sys_thread_tgt[0],
    dependencies : deps,
)
