project('deko3d-dkp', 'c', meson_version : '>= 1.7.0')

# Get `devkitpro` path option, otherwise use default
devkitpro = get_option('devkitpro')

# deko3d pacman package install the static library to `/opt/devkitpro/libnx` directory
# See: `pacman -Ql deko3d`
deko3d_libdir = devkitpro / 'libnx/lib'
deko3d_incdir = devkitpro / 'libnx/include'

# Release version
deko3d_dep = declare_dependency(
    include_directories : include_directories(deko3d_incdir),
    link_args : ['-L@0@'.format(deko3d_libdir), '-ldeko3d', '-lnx', '-lm'],
)

# Debug version
deko3dd_dep = declare_dependency(
    include_directories : include_directories(deko3d_incdir),
    link_args : ['-L@0@'.format(deko3d_libdir), '-ldeko3dd', '-lnx', '-lm'],
)
