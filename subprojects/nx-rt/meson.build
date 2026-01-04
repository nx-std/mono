project('nx-rt', version : '0.1.0')

cargo = find_program('cargo', required : true)

#---------------------------------------------------------------------------------
# Dependencies
#---------------------------------------------------------------------------------
# Rust dependencies here are just informative so Meson can build the dependencies in the correct order
# nx-alloc
nx_alloc_proj = subproject('nx-alloc')
nx_alloc_dep = nx_alloc_proj.get_variable('nx_alloc_dep')

# nx-panic-handler
nx_panic_handler_proj = subproject('nx-panic-handler')
nx_panic_handler_dep = nx_panic_handler_proj.get_variable('nx_panic_handler_dep')

# nx-rt-env
nx_rt_env_proj = subproject('nx-rt-env')
nx_rt_env_dep = nx_rt_env_proj.get_variable('nx_rt_env_dep')

# nx-svc
nx_svc_proj = subproject('nx-svc')
nx_svc_dep = nx_svc_proj.get_variable('nx_svc_dep')

# nx-std-sync
nx_std_sync_proj = subproject('nx-std-sync')
nx_std_sync_dep = nx_std_sync_proj.get_variable('nx_std_sync_dep')

# nx-sys-mem
nx_sys_mem_proj = subproject('nx-sys-mem')
nx_sys_mem_dep = nx_sys_mem_proj.get_variable('nx_sys_mem_dep')

# nx-sys-sync
nx_sys_sync_proj = subproject('nx-sys-sync')
nx_sys_sync_dep = nx_sys_sync_proj.get_variable('nx_sys_sync_dep')

# nx-sys-thread
nx_sys_thread_proj = subproject('nx-sys-thread')
nx_sys_thread_dep = nx_sys_thread_proj.get_variable('nx_sys_thread_dep')

# nx-sys-thread-tls
nx_sys_thread_tls_proj = subproject('nx-sys-thread-tls')
nx_sys_thread_tls_dep = nx_sys_thread_tls_proj.get_variable('nx_sys_thread_tls_dep')

# Dependencies list
deps = [
    nx_alloc_dep,
    nx_panic_handler_dep,
    nx_rt_env_dep,
    nx_std_sync_dep,
    nx_svc_dep,
    nx_sys_mem_dep,
    nx_sys_sync_dep,
    nx_sys_thread_dep,
    nx_sys_thread_tls_dep,
]

#---------------------------------------------------------------------------------
# Static library
#---------------------------------------------------------------------------------
# Target
nx_rt_tgt = custom_target(
    'nx-rt',
    command : [
        cargo, 'build',
        '--package', meson.project_name(),
        '--profile', get_option('buildtype') == 'release' ? 'release' : 'dev',
        '--target-dir', meson.global_build_root() / 'cargo-target',
        '--artifact-dir', '@OUTDIR@',
        '--features', 'ffi',
    ],
    output : ['libnx_rt.a', 'libnx_rt.rlib'],
    console : true,
    build_by_default : true,
    build_always_stale : true,
)

#---------------------------------------------------------------------------------
# Dependency declaration
#---------------------------------------------------------------------------------
# Linker script for overriding libnx runtime functions
nx_rt_ld_override = meson.current_source_dir() / 'rt_override.ld'

nx_rt_dep = declare_dependency(
    link_with : nx_rt_tgt[0],
    dependencies : deps,
)
