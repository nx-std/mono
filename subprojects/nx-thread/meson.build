project('nx-thread', version : '0.1.0')

cargo = find_program('cargo', required : true)

#------------------------------------------------
# Dependencies
#------------------------------------------------
# Rust dependencies here ar just informative so Meson can build the dependencies in the correct order
# nx-thread-tls
nx_thread_tls_proj = subproject('nx-thread-tls')
nx_thread_tls_dep = nx_thread_tls_proj.get_variable('nx_thread_tls_dep')

# Dependencies list
deps = [
    nx_thread_tls_dep,
]

#------------------------------------------------
# Static library
#------------------------------------------------
# Include directories
inc = include_directories('include')

# Target
nx_thread_tgt = custom_target(
    'nx-thread',
    command : [
        cargo, 'build',
        '--package', meson.project_name(),
        '--profile', get_option('buildtype') == 'release' ? 'release' : 'dev',
        '--target-dir', meson.global_build_root() / 'cargo-target',
        '--artifact-dir', '@OUTDIR@',
    ],
    output : ['libnx_thread.a', 'libnx_thread.rlib'],
    console: true,
    build_by_default : true,
    build_always_stale: true,
)

nx_thread_dep = declare_dependency(
    include_directories : inc,
    link_with : nx_thread_tgt[0],
    dependencies : deps,
)

# Linker script override for thread functions
nx_thread_ld_override = meson.current_source_dir() / 'thread_redirect.ld'
