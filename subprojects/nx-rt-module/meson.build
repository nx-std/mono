project('nx-rt-module', version : '0.1.0')

cargo = find_program('cargo', required : true)

#---------------------------------------------------------------------------------
# Dependencies
#---------------------------------------------------------------------------------
# Rust dependencies here are just informative so Meson can build the dependencies in the correct order
# nx-panic-handler
nx_panic_handler_proj = subproject('nx-panic-handler')
nx_panic_handler_dep = nx_panic_handler_proj.get_variable('nx_panic_handler_dep')

# nx-rt-core (the kind-agnostic runtime core)
nx_rt_core_proj = subproject('nx-rt-core')
nx_rt_core_dep = nx_rt_core_proj.get_variable('nx_rt_core_dep')

deps = [
    nx_panic_handler_dep,
    nx_rt_core_dep,
]

#---------------------------------------------------------------------------------
# Static library
#---------------------------------------------------------------------------------
# A dynamically loadable module owns no process entry point and overrides no
# libnx symbols of its own, so this crate ships neither a `_start` nor an
# `*_override.ld` fragment — there is no `nx_rt_module_ld_override` variable.
nx_rt_module_tgt = custom_target(
    'nx-rt-module',
    command : [
        cargo, 'build',
        '--package', meson.project_name(),
        '--profile', get_option('buildtype') == 'release' ? 'release' : 'dev',
        '--target-dir', meson.global_build_root() / 'cargo-target',
        '--artifact-dir', '@OUTDIR@',
    ],
    output : ['libnx_rt_module.rlib'],
    console : true,
    build_by_default : true,
    build_always_stale : true,
)

#---------------------------------------------------------------------------------
# Dependency declaration
#---------------------------------------------------------------------------------
nx_rt_module_dep = declare_dependency(
    sources : nx_rt_module_tgt,
    dependencies : deps,
)
