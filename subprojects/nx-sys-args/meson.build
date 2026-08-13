project('nx-sys-args', version : '0.1.0')

cargo = find_program('cargo', required : true)

#---------------------------------------------------------------------------------
# Dependencies
#---------------------------------------------------------------------------------
# Rust dependencies here are just informative so Meson can build the dependencies in the correct order
# nx-panic-handler
nx_panic_handler_proj = subproject('nx-panic-handler')
nx_panic_handler_dep = nx_panic_handler_proj.get_variable('nx_panic_handler_dep')

# nx-sys-sync
nx_sys_sync_proj = subproject('nx-sys-sync')
nx_sys_sync_dep = nx_sys_sync_proj.get_variable('nx_sys_sync_dep')

# Dependencies list
deps = [
    nx_panic_handler_dep,
    nx_sys_sync_dep,
]

#---------------------------------------------------------------------------------
# Static library
#---------------------------------------------------------------------------------
# Target
nx_sys_args_tgt = custom_target(
    'nx-sys-args',
    command : [
        cargo, 'build',
        '--package', meson.project_name(),
        '--profile', get_option('buildtype') == 'release' ? 'release' : 'dev',
        '--target-dir', meson.global_build_root() / 'cargo-target',
        '--artifact-dir', '@OUTDIR@',
    ],
    output : ['libnx_sys_args.rlib'],
    console : true,
    build_by_default : true,
    build_always_stale : true,
)

#---------------------------------------------------------------------------------
# Dependency declaration
#---------------------------------------------------------------------------------
nx_sys_args_dep = declare_dependency(
    sources : nx_sys_args_tgt,
    dependencies : deps,
)
