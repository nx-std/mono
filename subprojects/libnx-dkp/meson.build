project('nx-dkp', 'c', meson_version : '>= 1.3.0')

#---------------------------------------------------------------------------------
# Options
#---------------------------------------------------------------------------------
# The `use_nx` global option is used to enable/disable the function overrides.
# If enabled, and the specialized options are in auto mode, the overrides are enabled.  
# If disabled, and the specialized options are in auto mode, the overrides are disabled.
# Individually, the specialized options can override the `use_nx` global option. 
use_nx = get_option('use_nx')

use_nx_alloc = get_option('use_nx_alloc').enable_auto_if(use_nx.enabled()).disable_auto_if(use_nx.disabled())
use_nx_rand = get_option('use_nx_rand').enable_auto_if(use_nx.enabled()).disable_auto_if(use_nx.disabled())
use_nx_svc = get_option('use_nx_svc').enable_auto_if(use_nx.enabled()).disable_auto_if(use_nx.disabled())
use_nx_sys_sync = get_option('use_nx_sys_sync').enable_auto_if(use_nx.enabled()).disable_auto_if(use_nx.disabled())
use_nx_sys_thread = get_option('use_nx_sys_thread').enable_auto_if(use_nx.enabled()).disable_auto_if(use_nx.disabled())
use_nx_time = get_option('use_nx_time').enable_auto_if(use_nx.enabled()).disable_auto_if(use_nx.disabled())

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

# nx-std
if (use_nx.enabled() 
    or use_nx_alloc.enabled() or use_nx_rand.enabled() or use_nx_svc.enabled() 
    or use_nx_sys_sync.enabled() or use_nx_sys_thread.enabled() 
    or use_nx_time.enabled())
    
    nx_std_proj = subproject(
        'nx-std', 
        default_options : {
            'use_nx_alloc' : '@0@'.format(use_nx_alloc),
            'use_nx_rand' : '@0@'.format(use_nx_rand),
            'use_nx_svc' : '@0@'.format(use_nx_svc),
            'use_nx_sys_sync' : '@0@'.format(use_nx_sys_sync),
            'use_nx_sys_thread' : '@0@'.format(use_nx_sys_thread),
            'use_nx_time' : '@0@'.format(use_nx_time),
        },
    )


    debug('Using nx-std')
    nx_dep_deps += nx_std_proj.get_variable('nx_std_dep')
    nx_dep_link_args += nx_std_proj.get_variable('nx_std_dep_override_link_args')
endif


nx_dep = declare_dependency(
    include_directories : include_directories(nx_incdir),
    dependencies : nx_dep_deps,
    link_args : nx_dep_link_args,
)
