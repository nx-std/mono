project('nx-romfs', version : '0.1.0')

cargo = find_program('cargo', required : true)

#---------------------------------------------------------------------------------
# Dependencies
#---------------------------------------------------------------------------------
# Rust dependencies here are just informative so Meson can build the dependencies in the correct order
# nx-alloc
nx_alloc_proj = subproject('nx-alloc')
nx_alloc_dep = nx_alloc_proj.get_variable('nx_alloc_dep')

# nx-fsdev
nx_fsdev_proj = subproject('nx-fsdev')
nx_fsdev_dep = nx_fsdev_proj.get_variable('nx_fsdev_dep')

# nx-panic-handler
nx_panic_handler_proj = subproject('nx-panic-handler')
nx_panic_handler_dep = nx_panic_handler_proj.get_variable('nx_panic_handler_dep')

# nx-service-fs
nx_service_fs_proj = subproject('nx-service-fs')
nx_service_fs_dep = nx_service_fs_proj.get_variable('nx_service_fs_dep')

# nx-sf
nx_sf_proj = subproject('nx-sf')
nx_sf_dep = nx_sf_proj.get_variable('nx_sf_dep')

# nx-std-path
nx_std_path_proj = subproject('nx-std-path')
nx_std_path_dep = nx_std_path_proj.get_variable('nx_std_path_dep')

# nx-std-sync
nx_std_sync_proj = subproject('nx-std-sync')
nx_std_sync_dep = nx_std_sync_proj.get_variable('nx_std_sync_dep')

# nx-sys-fd
nx_sys_fd_proj = subproject('nx-sys-fd')
nx_sys_fd_dep = nx_sys_fd_proj.get_variable('nx_sys_fd_dep')

# Dependencies list
deps = [
    nx_alloc_dep,
    nx_fsdev_dep,
    nx_panic_handler_dep,
    nx_service_fs_dep,
    nx_sf_dep,
    nx_std_path_dep,
    nx_std_sync_dep,
    nx_sys_fd_dep,
]

#---------------------------------------------------------------------------------
# Static library
#---------------------------------------------------------------------------------
# Target
nx_romfs_tgt = custom_target(
    'nx-romfs',
    command : [
        cargo, 'build',
        '--package', meson.project_name(),
        '--profile', get_option('buildtype') == 'release' ? 'release' : 'dev',
        '--target-dir', meson.global_build_root() / 'cargo-target',
        '--artifact-dir', '@OUTDIR@',
    ],
    output : ['libnx_romfs.rlib'],
    console : true,
    build_by_default : true,
    build_always_stale : true,
)

#---------------------------------------------------------------------------------
# Dependency declaration
#---------------------------------------------------------------------------------
# Linker script for overriding libnx's romfs devoptab layer
nx_romfs_ld_override = meson.current_source_dir() / 'romfs_override.ld'

nx_romfs_dep = declare_dependency(
    sources : nx_romfs_tgt,
    dependencies : deps,
)
