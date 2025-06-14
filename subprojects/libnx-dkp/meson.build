project('nx-dkp', 'c', meson_version : '>= 1.3.0')

#---------------------------------------------------------------------------------
# Static library
#---------------------------------------------------------------------------------
# Get `devkitpro` path option, otherwise use default
devkitpro = get_option('devkitpro')

# File paths
nx_libdir = devkitpro / 'libnx/lib'
nx_incdir = devkitpro / 'libnx/include'
nx_switch_specs = devkitpro / 'libnx/switch.specs'
nx_default_icon = devkitpro / 'libnx/default_icon.jpg'

#---------------------------------------------------------------------------------
# Dependency declaration
#---------------------------------------------------------------------------------

nx_dep_deps = []
nx_dep_link_args = ['-L@0@'.format(nx_libdir), '-lnx']

# Override SVC functions and propagate nx-svc dependency
if get_option('use_nx_svc').enabled()
    nx_svc_proj = subproject('nx-svc')

    nx_dep_deps += nx_svc_proj.get_variable('nx_svc_dep')
    nx_dep_link_args += ['-T', nx_svc_proj.get_variable('nx_svc_ld_override')]
endif

# Override sync functions and propagate nx-sync dependency
if get_option('use_nx_sync').enabled()
    nx_sync_proj = subproject('nx-sync')

    nx_dep_deps += nx_sync_proj.get_variable('nx_sync_dep')
    nx_dep_link_args += ['-T', nx_sync_proj.get_variable('nx_sync_ld_override')]
endif

# Override thread functions and propagate nx-thread dependency
if get_option('use_nx_thread').enabled()
    nx_thread_proj = subproject('nx-thread')

    nx_dep_deps += nx_thread_proj.get_variable('nx_thread_dep')
    nx_dep_link_args += ['-T', nx_thread_proj.get_variable('nx_thread_ld_override')]
endif

# Override thread functions and propagate nx-thread dependency
if get_option('use_nx_thread_tls').enabled()
    nx_thread_tls_proj = subproject('nx-thread-tls')

    nx_dep_deps += nx_thread_tls_proj.get_variable('nx_thread_tls_dep')
    nx_dep_link_args += ['-T', nx_thread_tls_proj.get_variable('nx_thread_tls_ld_override')]
endif

# Override time functions and propagate nx-time dependency
if get_option('use_nx_time').enabled()
    nx_time_proj = subproject('nx-time')

    nx_dep_deps += nx_time_proj.get_variable('nx_time_dep')
    nx_dep_link_args += ['-T', nx_time_proj.get_variable('nx_time_ld_override')]
endif

# Override random functions and propagate nx-rand dependency
if get_option('use_nx_rand').enabled()
    nx_rand_proj = subproject('nx-rand')

    nx_dep_deps += nx_rand_proj.get_variable('nx_rand_dep')
    nx_dep_link_args += ['-T', nx_rand_proj.get_variable('nx_rand_ld_override')]
endif

nx_dep = declare_dependency(
    include_directories : include_directories(nx_incdir),
    dependencies : nx_dep_deps,
    link_args : nx_dep_link_args,
)
