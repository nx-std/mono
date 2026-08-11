project('nx-sys-fd', version : '0.1.0')

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

# nx-std-path
nx_std_path_proj = subproject('nx-std-path')
nx_std_path_dep = nx_std_path_proj.get_variable('nx_std_path_dep')

# nx-sys-sync
nx_sys_sync_proj = subproject('nx-sys-sync')
nx_sys_sync_dep = nx_sys_sync_proj.get_variable('nx_sys_sync_dep')

# nx-sys-thread-tls
nx_sys_thread_tls_proj = subproject('nx-sys-thread-tls')
nx_sys_thread_tls_dep = nx_sys_thread_tls_proj.get_variable('nx_sys_thread_tls_dep')

# Dependencies list
deps = [
    nx_alloc_dep,
    nx_panic_handler_dep,
    nx_std_path_dep,
    nx_sys_sync_dep,
    nx_sys_thread_tls_dep,
]

#---------------------------------------------------------------------------------
# Static library
#---------------------------------------------------------------------------------
# Target
nx_sys_fd_tgt = custom_target(
    'nx-sys-fd',
    command : [
        cargo, 'build',
        '--package', meson.project_name(),
        '--profile', get_option('buildtype') == 'release' ? 'release' : 'dev',
        '--target-dir', meson.global_build_root() / 'cargo-target',
        '--artifact-dir', '@OUTDIR@',
    ],
    output : ['libnx_sys_fd.rlib'],
    console : true,
    build_by_default : true,
    build_always_stale : true,
)

#---------------------------------------------------------------------------------
# Dependency declaration
#---------------------------------------------------------------------------------
# Linker script for overriding libsysbase file descriptor table functions
nx_sys_fd_ld_override = meson.current_source_dir() / 'sys_fd_override.ld'

nx_sys_fd_dep = declare_dependency(
    sources : nx_sys_fd_tgt,
    dependencies : deps,
)
