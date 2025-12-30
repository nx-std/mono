project('switchbrew', meson_version: '>= 1.7.0')

## Subprojects
# Libraries
subproject('sysroot-dkp')

if get_option('use_libnx_dkp').disabled()
    subproject('libnx')
else
    subproject('libnx-dkp')
endif

# Executables
subproject('tests')
subproject('examples')
subproject('nx-hbmenu')
