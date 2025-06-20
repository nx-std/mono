project('nx-std', version : '0.1.0', meson_version : '>= 1.1.0')

cargo = find_program('cargo', required : true)

#------------------------------------------------
# Dependencies
#------------------------------------------------
deps = []
deps_override_link_args = []
deps_cargo_features = []

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
    deps_override_link_args += ['-T', nx_std_sync_proj.get_variable('nx_std_sync_ld_override')]

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

# nx-thread
if get_option('use_nx_thread').enabled()
    nx_thread_proj = subproject('nx-thread')

    deps += nx_thread_proj.get_variable('nx_thread_dep')
    deps_override_link_args += ['-T', nx_thread_proj.get_variable('nx_thread_ld_override')]

    debug('thread feature: enabled')
    deps_cargo_features += ['thread']
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

#------------------------------------------------
# Static library
#------------------------------------------------
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
    console: true,
    build_by_default : true,
    build_always_stale: true,
)

#------------------------------------------------
# Dependency declaration
#------------------------------------------------
nx_std_dep = declare_dependency(
    include_directories : inc,
    link_with : nx_std_tgt[0],
    dependencies : deps,
)

nx_std_dep_override_link_args = deps_override_link_args
