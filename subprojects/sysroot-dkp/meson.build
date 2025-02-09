project('sysroot-dkp', 'c')

# Get `devkitpro` path option, otherwise use default
devkitpro = get_option('devkitpro')

# File paths
sysroot = devkitpro / 'devkitA64/aarch64-none-elf'
sysroot_libdir = sysroot / 'lib'
sysroot_incdir = sysroot / 'include'

sysroot_dep = declare_dependency(
    include_directories : include_directories(sysroot_incdir),
    link_args : ['--sysroot=@0@'.format(sysroot)],
)
