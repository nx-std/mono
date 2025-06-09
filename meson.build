project('switchbrew', meson_version : '>=1.4.0')

## Subprojects
# Libraries
subproject('sysroot-dkp')
subproject('nx-cpu')
subproject('nx-svc')
subproject('nx-sync')
subproject('nx-thread')
subproject('nx-time')

if get_option('use_libnx_dkp').enabled()
    subproject('libnx-dkp')
else
    subproject('libnx')
endif

# Executables
subproject('tests')
subproject('examples')
subproject('nx-hbmenu')
