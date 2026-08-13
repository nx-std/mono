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

# nx-std-fs
nx_std_fs_proj = subproject('nx-std-fs')
nx_std_fs_dep = nx_std_fs_proj.get_variable('nx_std_fs_dep')

# nx-std-path
nx_std_path_proj = subproject('nx-std-path')
nx_std_path_dep = nx_std_path_proj.get_variable('nx_std_path_dep')

# nx-std-sync
nx_std_sync_proj = subproject('nx-std-sync')
nx_std_sync_dep = nx_std_sync_proj.get_variable('nx_std_sync_dep')

# nx-sys-net
nx_sys_net_proj = subproject('nx-sys-net')
nx_sys_net_dep = nx_sys_net_proj.get_variable('nx_sys_net_dep')

# Dependencies list
deps = [
    nx_alloc_dep,
    nx_panic_handler_dep,
    nx_std_fs_dep,
    nx_std_path_dep,
    nx_std_sync_dep,
    nx_sys_net_dep,
]

#---------------------------------------------------------------------------------
# Static library
#---------------------------------------------------------------------------------
inc = include_directories('include')

# Target
#
# Built with `ffi` here, unlike every other producer in this workspace. The umbrella crate is what
# normally turns a producer's C surface on, and nothing there depends on this one: a test binary
# links it directly. It is also the one Rust static library such a binary links, which is what the
# `extern-state` on its dependencies is for -- the socket driver and descriptor table it reaches are
# the ones the program already brought up. See `docs/code/rust-process-wide-state.md`.
nx_tests_framework_tgt = custom_target(
    'nx-tests-framework',
    command : [
        cargo, 'build',
        '--package', 'nx-tests-framework',
        '--profile', get_option('buildtype') == 'release' ? 'release' : 'dev',
        '--target-dir', meson.global_build_root() / 'cargo-target',
        '--artifact-dir', '@OUTDIR@',
        '--features', 'ffi',
    ],
    output : ['libnx_tests_framework.a', 'libnx_tests_framework.rlib'],
    console : true,
    build_by_default : true,
    build_always_stale : true,
)

#---------------------------------------------------------------------------------
# Dependency declaration
#---------------------------------------------------------------------------------
nx_tests_framework_dep = declare_dependency(
    include_directories : inc,
    sources : nx_tests_framework_tgt,
    link_with : nx_tests_framework_tgt[0],
    dependencies : deps,
)
