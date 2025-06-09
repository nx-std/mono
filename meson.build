project('switchbrew', meson_version : '>=1.4.0')

## Subprojects
# Libraries
subproject('sysroot-dkp')

if get_option('use_libnx_dkp').disabled()
    subproject('libnx')
else
    subproject('libnx-dkp')
endif

if get_option('use_nx_cpu').disabled()
    subproject('nx-cpu')
endif
if get_option('use_nx_svc').disabled()
    subproject('nx-svc')
endif
if get_option('use_nx_thread').disabled()
    subproject('nx-thread')
endif
if get_option('use_nx_sync').disabled()
    subproject('nx-sync')
endif
if get_option('use_nx_time').disabled()
    subproject('nx-time')
endif

# Executables
subproject('tests')
subproject('examples')
subproject('nx-hbmenu')
