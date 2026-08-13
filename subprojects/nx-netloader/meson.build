project('nx-netloader', version : '0.1.0')

cargo = find_program('cargo', required : true)

#---------------------------------------------------------------------------------
# Dependencies
#---------------------------------------------------------------------------------
# Rust dependencies here are just informative so Meson can build the dependencies in the correct order
# nx-alloc
nx_alloc_proj = subproject('nx-alloc')
nx_alloc_dep = nx_alloc_proj.get_variable('nx_alloc_dep')

# nx-cpu
nx_cpu_proj = subproject('nx-cpu')
nx_cpu_dep = nx_cpu_proj.get_variable('nx_cpu_dep')

# nx-panic-handler
nx_panic_handler_proj = subproject('nx-panic-handler')
nx_panic_handler_dep = nx_panic_handler_proj.get_variable('nx_panic_handler_dep')

# nx-std
nx_std_proj = subproject('nx-std')
nx_std_dep = nx_std_proj.get_variable('nx_std_dep')

# nx-std-path
nx_std_path_proj = subproject('nx-std-path')
nx_std_path_dep = nx_std_path_proj.get_variable('nx_std_path_dep')

# nx-svc
nx_svc_proj = subproject('nx-svc')
nx_svc_dep = nx_svc_proj.get_variable('nx_svc_dep')

# nx-sys-net
nx_sys_net_proj = subproject('nx-sys-net')
nx_sys_net_dep = nx_sys_net_proj.get_variable('nx_sys_net_dep')

# Dependencies list
deps = [
    nx_alloc_dep,
    nx_cpu_dep,
    nx_panic_handler_dep,
    nx_std_dep,
    nx_std_path_dep,
    nx_svc_dep,
    nx_sys_net_dep,
]

#---------------------------------------------------------------------------------
# Static library
#---------------------------------------------------------------------------------
inc = include_directories('include')

# Target
nx_netloader_tgt = custom_target(
    'nx-netloader',
    command : [
        cargo, 'build',
        '--package', meson.project_name(),
        '--profile', get_option('buildtype') == 'release' ? 'release' : 'dev',
        '--target-dir', meson.global_build_root() / 'cargo-target',
        '--artifact-dir', '@OUTDIR@',
        # The C surface is what this crate exists to expose, and nothing upstream turns the feature
        # on for it: the umbrella crate enables its dependencies' `ffi` features, and this one is
        # linked directly rather than through it.
        '--features', 'ffi',
    ],
    output : ['libnx_netloader.a', 'libnx_netloader.rlib'],
    console : true,
    build_by_default : true,
    build_always_stale : true,
)

#---------------------------------------------------------------------------------
# Dependency declaration
#---------------------------------------------------------------------------------
nx_netloader_dep = declare_dependency(
    include_directories : inc,
    sources : nx_netloader_tgt,
    link_with : nx_netloader_tgt[0],
    dependencies : deps,
)
