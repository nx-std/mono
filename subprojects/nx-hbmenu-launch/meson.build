project('nx-hbmenu-launch', version : '0.1.0')

cargo = find_program('cargo', required : true)

## Static library
# Target
nx_hbmenu_launch_tgt = custom_target(
    'nx-hbmenu-launch',
    command : [
        cargo, 'build',
        '--package', meson.project_name(),
        '--profile', get_option('buildtype') == 'release' ? 'release' : 'dev',
        '--target-dir', meson.global_build_root() / 'cargo-target',
        '--artifact-dir', '@OUTDIR@',
    ],
    output : ['libnx_hbmenu_launch.a', 'libnx_hbmenu_launch.rlib'],
    console : true,
    build_by_default : true,
    build_always_stale : true,
)

#---------------------------------------------------------------------------------
# Dependency declaration
#---------------------------------------------------------------------------------
# Linker script for overriding hbmenu launch functions
nx_hbmenu_launch_ld_override = meson.current_source_dir() / 'hbmenu_launch_override.ld'

nx_hbmenu_launch_dep = declare_dependency(
    link_with : nx_hbmenu_launch_tgt[0],
    link_args : ['-T', nx_hbmenu_launch_ld_override],
)