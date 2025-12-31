project('nx-cpu', version : '0.1.0')

cargo = find_program('cargo', required : true)

#---------------------------------------------------------------------------------
# Dependencies
#---------------------------------------------------------------------------------
# Rust dependencies here are just informative so Meson can build the dependencies in the correct order
# nx-panic-handler
nx_panic_handler_proj = subproject('nx-panic-handler')
nx_panic_handler_dep = nx_panic_handler_proj.get_variable('nx_panic_handler_dep')

# Dependencies list
deps = [
    nx_panic_handler_dep,
]

#---------------------------------------------------------------------------------
# Static library
#---------------------------------------------------------------------------------
# Target
nx_cpu_tgt = custom_target(
    'nx-cpu',
    command : [
        cargo, 'build',
        '--package', meson.project_name(),
        '--profile', get_option('buildtype') == 'release' ? 'release' : 'dev',
        '--target-dir', meson.global_build_root() / 'cargo-target',
        '--artifact-dir', '@OUTDIR@',
    ],
    output : ['libnx_cpu.a', 'libnx_cpu.rlib'],
    console : true,
    build_by_default : true,
    build_always_stale : true,
)

nx_cpu_dep = declare_dependency(
    link_with : nx_cpu_tgt[0],
    dependencies : deps,
)
