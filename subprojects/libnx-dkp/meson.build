project('nx-dkp', 'c', meson_version : '>= 1.7.0')

#---------------------------------------------------------------------------------
# Features
#---------------------------------------------------------------------------------
# Whether any `use_nx_*` feature is on, which is the only thing this archive asks
# of the feature set: it decides whether `nx-std` is linked in at all.
#
# Which features are on is resolved by `nx-std` itself, and so is the subset this
# archive can wire: the prebuilt libnx brings its own runtime and services, so a
# feature that would alias over them is unavailable rather than merely disabled.
# Nothing here has to mask them out again, and asking about all of them is what
# makes `-Duse_nx_service_vi=enabled` against this archive reach the refusal in
# `nx-std` instead of being quietly dropped on the way.
#
# The dependency edges between features are not applied here and do not need to
# be: pull-up only ever enables a feature behind one that is already enabled, so
# it cannot turn this answer from false to true, and push-down only retracts a
# feature the master switch turned on, which already answered true.
use_nx = get_option('use_nx')

nx_features_asked = {
    'alloc' : get_option('use_nx_alloc'),
    'display' : get_option('use_nx_display'),
    'fsdev' : get_option('use_nx_fsdev'),
    'pm' : get_option('use_nx_pm'),
    'rand' : get_option('use_nx_rand'),
    'rt' : get_option('use_nx_rt'),
    'service_apm' : get_option('use_nx_service_apm'),
    'service_applet' : get_option('use_nx_service_applet'),
    'service_applet_err' : get_option('use_nx_service_applet_err'),
    'service_fs' : get_option('use_nx_service_fs'),
    'service_hid' : get_option('use_nx_service_hid'),
    'service_nv' : get_option('use_nx_service_nv'),
    'service_set' : get_option('use_nx_service_set'),
    'service_sfdnsres' : get_option('use_nx_service_sfdnsres'),
    'service_sm' : get_option('use_nx_service_sm'),
    'service_time' : get_option('use_nx_service_time'),
    'service_vi' : get_option('use_nx_service_vi'),
    'sf' : get_option('use_nx_sf'),
    'std_sync' : get_option('use_nx_std_sync'),
    'svc' : get_option('use_nx_svc'),
    'sys_fd' : get_option('use_nx_sys_fd'),
    'sys_mem' : get_option('use_nx_sys_mem'),
    'sys_sync' : get_option('use_nx_sys_sync'),
    'sys_thread' : get_option('use_nx_sys_thread'),
    'sys_thread_tls' : get_option('use_nx_sys_thread_tls'),
    'time' : get_option('use_nx_time'),
    'wlaninf' : get_option('use_nx_wlaninf'),
}

nx_features_any = use_nx.enabled()
foreach _, asked : nx_features_asked
    nx_features_any = nx_features_any or asked.enabled()
endforeach

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
if nx_features_any
    nx_std_proj = subproject('nx-std')

    debug('Using nx-std')
    nx_dep_deps += nx_std_proj.get_variable('nx_std_dep')
    nx_dep_link_args += nx_std_proj.get_variable('nx_std_dep_override_link_args')
endif


nx_dep = declare_dependency(
    include_directories : include_directories(nx_incdir),
    dependencies : nx_dep_deps,
    link_args : nx_dep_link_args,
)
