project('nx-std', version : '0.1.0', meson_version : '>= 1.7.0')

cargo = find_program('cargo', required : true)

#---------------------------------------------------------------------------------
# Dependencies
#---------------------------------------------------------------------------------
deps = []
deps_override_link_args = []
deps_cargo_features = []

# nx-panic-handler (always required)
nx_panic_handler_proj = subproject('nx-panic-handler')
deps += nx_panic_handler_proj.get_variable('nx_panic_handler_dep')

# nx-alloc
if get_option('use_nx_alloc').enabled()
    nx_alloc_proj = subproject('nx-alloc')

    deps += nx_alloc_proj.get_variable('nx_alloc_dep')
    deps_override_link_args += ['-T', nx_alloc_proj.get_variable('nx_alloc_ld_override')]

    debug('alloc feature: enabled')
    deps_cargo_features += ['alloc']
endif

# nx-rand
if get_option('use_nx_rand').enabled()
    nx_rand_proj = subproject('nx-rand')

    deps += nx_rand_proj.get_variable('nx_rand_dep')
    deps_override_link_args += ['-T', nx_rand_proj.get_variable('nx_rand_ld_override')]

    debug('rand feature: enabled')
    deps_cargo_features += ['rand']
endif

# nx-std-sync
if get_option('use_nx_std_sync').enabled()
    nx_std_sync_proj = subproject('nx-std-sync')

    deps += nx_std_sync_proj.get_variable('nx_std_sync_dep')

    debug('sync feature: enabled')
    deps_cargo_features += ['sync']
endif

# nx-svc
if get_option('use_nx_svc').enabled()
    nx_svc_proj = subproject('nx-svc')

    deps += nx_svc_proj.get_variable('nx_svc_dep')
    deps_override_link_args += ['-T', nx_svc_proj.get_variable('nx_svc_ld_override')]

    debug('svc feature: enabled')
    deps_cargo_features += ['svc']
endif

# nx-sys-sync
if get_option('use_nx_sys_sync').enabled()
    nx_sys_sync_proj = subproject('nx-sys-sync')

    deps += nx_sys_sync_proj.get_variable('nx_sys_sync_dep')
    deps_override_link_args += ['-T', nx_sys_sync_proj.get_variable('nx_sys_sync_ld_override')]

    debug('sys-sync feature: enabled')
    deps_cargo_features += ['sys-sync']
endif

# nx-sys-thread-tls
if get_option('use_nx_sys_thread_tls').enabled()
    nx_sys_thread_tls_proj = subproject('nx-sys-thread-tls')

    deps += nx_sys_thread_tls_proj.get_variable('nx_sys_thread_tls_dep')
    deps_override_link_args += ['-T', nx_sys_thread_tls_proj.get_variable('nx_sys_thread_tls_ld_override')]

    debug('sys-thread-tls feature: enabled')
    deps_cargo_features += ['sys-thread-tls']
endif

# nx-sys-mem
if get_option('use_nx_sys_mem').enabled()
    nx_sys_mem_proj = subproject('nx-sys-mem')

    deps += nx_sys_mem_proj.get_variable('nx_sys_mem_dep')
    deps_override_link_args += ['-T', nx_sys_mem_proj.get_variable('nx_sys_mem_ld_override')]

    debug('sys-mem feature: enabled')
    deps_cargo_features += ['sys-mem']
endif

# nx-sys-thread
if get_option('use_nx_sys_thread').enabled()
    nx_sys_thread_proj = subproject('nx-sys-thread')

    deps += nx_sys_thread_proj.get_variable('nx_sys_thread_dep')
    deps_override_link_args += ['-T', nx_sys_thread_proj.get_variable('nx_sys_thread_ld_override')]

    debug('sys-thread feature: enabled')
    deps_cargo_features += ['sys-thread']
endif

# nx-time
if get_option('use_nx_time').enabled()
    nx_time_proj = subproject('nx-time')

    deps += nx_time_proj.get_variable('nx_time_dep')
    deps_override_link_args += ['-T', nx_time_proj.get_variable('nx_time_ld_override')]

    debug('time feature: enabled')
    deps_cargo_features += ['time']
endif

# nx-rt
if get_option('use_nx_rt').enabled()
    nx_rt_proj = subproject('nx-rt')

    deps += nx_rt_proj.get_variable('nx_rt_dep')
    deps_override_link_args += ['-T', nx_rt_proj.get_variable('nx_rt_ld_override')]

    debug('rt feature: enabled')
    deps_cargo_features += ['rt']
endif

# nx-sf
if get_option('use_nx_sf').enabled()
    nx_sf_proj = subproject('nx-sf')

    deps += nx_sf_proj.get_variable('nx_sf_dep')
    deps_override_link_args += ['-T', nx_sf_proj.get_variable('nx_sf_ld_override')]

    debug('sf feature: enabled')
    deps_cargo_features += ['sf']
endif

# nx-service-apm
if get_option('use_nx_service_apm').enabled()
    nx_service_apm_proj = subproject('nx-service-apm')

    deps += nx_service_apm_proj.get_variable('nx_service_apm_dep')

    debug('service-apm feature: enabled')
    deps_cargo_features += ['service-apm']
endif

# nx-service-applet
if get_option('use_nx_service_applet').enabled()
    nx_service_applet_proj = subproject('nx-service-applet')

    deps += nx_service_applet_proj.get_variable('nx_service_applet_dep')

    debug('service-applet feature: enabled')
    deps_cargo_features += ['service-applet']
endif

# nx-service-hid
if get_option('use_nx_service_hid').enabled()
    nx_service_hid_proj = subproject('nx-service-hid')

    deps += nx_service_hid_proj.get_variable('nx_service_hid_dep')

    debug('service-hid feature: enabled')
    deps_cargo_features += ['service-hid']
endif

# nx-service-set
if get_option('use_nx_service_set').enabled()
    nx_service_set_proj = subproject('nx-service-set')

    deps += nx_service_set_proj.get_variable('nx_service_set_dep')

    debug('service-set feature: enabled')
    deps_cargo_features += ['service-set']
endif

# nx-service-sm
if get_option('use_nx_service_sm').enabled()
    nx_service_sm_proj = subproject('nx-service-sm')

    deps += nx_service_sm_proj.get_variable('nx_service_sm_dep')

    debug('service-sm feature: enabled')
    deps_cargo_features += ['service-sm']
endif

#---------------------------------------------------------------------------------
# Static library
#---------------------------------------------------------------------------------
# Include directories
inc = include_directories('include')

# Target
nx_std_tgt = custom_target(
    'nx-std',
    command : [
        cargo, 'build',
        '--package', meson.project_name(),
        '--profile', get_option('buildtype') == 'release' ? 'release' : 'dev',
        '--target-dir', meson.global_build_root() / 'cargo-target',
        '--artifact-dir', '@OUTDIR@',
        '--no-default-features',
        '--features', ','.join(['ffi'] + deps_cargo_features),
    ],
    output : ['libnx_std.a', 'libnx_std.rlib'],
    console : true,
    build_by_default : true,
    build_always_stale : true,
)

#---------------------------------------------------------------------------------
# Dependency declaration
#---------------------------------------------------------------------------------
nx_std_dep = declare_dependency(
    include_directories : inc,
    link_with : nx_std_tgt[0],
    dependencies : deps,
)

nx_std_dep_override_link_args = deps_override_link_args
