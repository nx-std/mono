project('nx-sys-virtmem', version : '0.1.0')

cargo = find_program('cargo', required : true)

#---------------------------------------------------------------------------------
# Dependencies
#---------------------------------------------------------------------------------
# Rust dependencies here are just informative so Meson can build the dependencies in the correct order
# nx-panic-handler
nx_panic_handler_proj = subproject('nx-panic-handler')
nx_panic_handler_dep = nx_panic_handler_proj.get_variable('nx_panic_handler_dep')

# nx-rand
nx_rand_proj = subproject('nx-rand')
nx_rand_dep = nx_rand_proj.get_variable('nx_rand_dep')

# nx-svc
nx_svc_proj = subproject('nx-svc')
nx_svc_dep = nx_svc_proj.get_variable('nx_svc_dep')

# nx-sys-sync
nx_sys_sync_proj = subproject('nx-sys-sync')
nx_sys_sync_dep = nx_sys_sync_proj.get_variable('nx_sys_sync_dep')

# Dependencies list
deps = [
    nx_panic_handler_dep,
    nx_rand_dep,
    nx_svc_dep,
    nx_sys_sync_dep,
]

#---------------------------------------------------------------------------------
# Static library
#---------------------------------------------------------------------------------
# Include directories
inc = include_directories('include')

# Target
nx_sys_virtmem_tgt = custom_target(
    'nx-sys-virtmem',
    command : [
        cargo, 'build',
        '--package', meson.project_name(),
        '--profile', get_option('buildtype') == 'release' ? 'release' : 'dev',
        '--target-dir', meson.global_build_root() / 'cargo-target',
        '--artifact-dir', '@OUTDIR@',
    ],
    output : ['libnx_sys_virtmem.rlib'],
    console : true,
    build_by_default : true,
    build_always_stale : true,
)

#---------------------------------------------------------------------------------
# Dependency declaration
#---------------------------------------------------------------------------------
# Linker script for overriding libnx functions
nx_sys_virtmem_ld_override = meson.current_source_dir() / 'sys_virtmem_override.ld'

nx_sys_virtmem_dep = declare_dependency(
    include_directories : inc,
    sources : nx_sys_virtmem_tgt,
    dependencies : deps,
)
