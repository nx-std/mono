project('nx-dkp', 'c', meson_version : '>= 1.3.0')

# Get `devkitpro` path option, otherwise use default
devkitpro = get_option('devkitpro')

# File paths
nx_libdir = devkitpro / 'libnx/lib'
nx_incdir = devkitpro / 'libnx/include'
nx_switch_specs = devkitpro / 'libnx/switch.specs'
nx_default_icon = devkitpro / 'libnx/default_icon.jpg'

nx_dep = declare_dependency(
    include_directories : include_directories(nx_incdir),
    link_args : ['-L@0@'.format(nx_libdir), '-lnx'],
)
