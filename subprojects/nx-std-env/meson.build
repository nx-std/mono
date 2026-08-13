project('nx-std-env', version : '0.1.0')

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

# nx-sys-args
nx_sys_args_proj = subproject('nx-sys-args')
nx_sys_args_dep = nx_sys_args_proj.get_variable('nx_sys_args_dep')

# nx-sys-env
nx_sys_env_proj = subproject('nx-sys-env')
nx_sys_env_dep = nx_sys_env_proj.get_variable('nx_sys_env_dep')

# Dependencies list
deps = [
    nx_alloc_dep,
    nx_panic_handler_dep,
    nx_std_path_dep,
    nx_sys_args_dep,
    nx_sys_env_dep,
]

#---------------------------------------------------------------------------------
# Static library
#---------------------------------------------------------------------------------
# Target
nx_std_env_tgt = custom_target(
    'nx-std-env',
    command : [
        cargo, 'build',
        '--package', meson.project_name(),
        '--profile', get_option('buildtype') == 'release' ? 'release' : 'dev',
        '--target-dir', meson.global_build_root() / 'cargo-target',
        '--artifact-dir', '@OUTDIR@',
    ],
    output : ['libnx_std_env.rlib'],
    console : true,
    build_by_default : true,
    build_always_stale : true,
)

#---------------------------------------------------------------------------------
# Dependency declaration
#---------------------------------------------------------------------------------
nx_std_env_dep = declare_dependency(
    sources : nx_std_env_tgt,
    dependencies : deps,
)
