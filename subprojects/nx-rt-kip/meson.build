project('nx-rt-kip', version : '0.1.0')

cargo = find_program('cargo', required : true)

#---------------------------------------------------------------------------------
# Options
#---------------------------------------------------------------------------------
# rt-link — the rustc-link pipeline's per-kind kernel-launch `.crt0`.
# On the rustc pipeline this crate supplies its own `_start`; on the GCC
# pipeline `_start` comes from libnx's `switch_crt0.s`, so the feature stays
# off there to avoid a duplicate-`_start` link error.
cargo_features = []
if get_option('link_pipeline') == 'rustc'
    cargo_features += 'rt-link'
endif

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
# A boot-time KIP is launched by the kernel and reads its capabilities from the
# KIP header, so this crate ships no `*_override.ld` fragment and there is no
# `nx_rt_kip_ld_override` variable. It does own a `_start`: the kernel-launch
# `.crt0` in `src/crt0.s`, emitted only on the `rustc`-link pipeline (the
# `rt-link` feature, enabled above when `link_pipeline == 'rustc'`).
cargo_cmd = [
    cargo, 'build',
    '--package', meson.project_name(),
    '--profile', get_option('buildtype') == 'release' ? 'release' : 'dev',
    '--target-dir', meson.global_build_root() / 'cargo-target',
    '--artifact-dir', '@OUTDIR@',
    '--no-default-features',
]
if cargo_features.length() > 0
    cargo_cmd += ['--features', ','.join(cargo_features)]
endif

nx_rt_kip_tgt = custom_target(
    'nx-rt-kip',
    command : cargo_cmd,
    output : ['libnx_rt_kip.rlib'],
    console : true,
    build_by_default : true,
    build_always_stale : true,
)

#---------------------------------------------------------------------------------
# Dependency declaration
#---------------------------------------------------------------------------------
nx_rt_kip_dep = declare_dependency(
    sources : nx_rt_kip_tgt,
    dependencies : deps,
)
