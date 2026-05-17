project('nx-sys-thread', 'c', version : '0.1.0')

cargo = find_program('cargo', required : true)

#---------------------------------------------------------------------------------
# Dependencies
#---------------------------------------------------------------------------------
# Rust dependencies here are just informative so Meson can build the dependencies in the correct order
# nx-panic-handler
nx_panic_handler_proj = subproject('nx-panic-handler')
nx_panic_handler_dep = nx_panic_handler_proj.get_variable('nx_panic_handler_dep')

# nx-alloc
nx_alloc_proj = subproject('nx-alloc')
nx_alloc_dep = nx_alloc_proj.get_variable('nx_alloc_dep')

# nx-svc
nx_svc_proj = subproject('nx-svc')
nx_svc_dep = nx_svc_proj.get_variable('nx_svc_dep')

# nx-sys-mem
nx_sys_mem_proj = subproject('nx-sys-mem')
nx_sys_mem_dep = nx_sys_mem_proj.get_variable('nx_sys_mem_dep')

# nx-sys-sync
nx_sys_sync_proj = subproject('nx-sys-sync')
nx_sys_sync_dep = nx_sys_sync_proj.get_variable('nx_sys_sync_dep')

# nx-sys-thread-tls
nx_sys_thread_tls_proj = subproject('nx-sys-thread-tls')
nx_sys_thread_tls_dep = nx_sys_thread_tls_proj.get_variable('nx_sys_thread_tls_dep')

# Dependencies list
deps = [
    nx_panic_handler_dep,
    nx_alloc_dep,
    nx_svc_dep,
    nx_sys_mem_dep,
    nx_sys_sync_dep,
    nx_sys_thread_tls_dep,
]

#---------------------------------------------------------------------------------
# C support shim
#---------------------------------------------------------------------------------
# newlib `_reent` provisioning for spawned threads (Task 6.6): `sizeof(struct
# _reent)` and the `_REENT_INIT_PTR` initializer are devkitA64 newlib ABI
# details the Rust core cannot reproduce. The archive is linked into the final
# NRO only when a consumer enables the `nx-sys-thread` FFI surface; with the
# surface off the Rust side references neither symbol, so the linker drops it.
nx_sys_thread_reent_lib = static_library(
    'nx_sys_thread_reent',
    'csrc/reent_shim.c',
    c_args : ['-D__SWITCH__', '-ffunction-sections', '-fdata-sections'],
)

#---------------------------------------------------------------------------------
# Static library
#---------------------------------------------------------------------------------
# Target
nx_sys_thread_tgt = custom_target(
    'nx-sys-thread',
    command : [
        cargo, 'build',
        '--package', meson.project_name(),
        '--profile', get_option('buildtype') == 'release' ? 'release' : 'dev',
        '--target-dir', meson.global_build_root() / 'cargo-target',
        '--artifact-dir', '@OUTDIR@',
    ],
    output : ['libnx_sys_thread.rlib'],
    console : true,
    build_by_default : true,
    build_always_stale : true,
)

#---------------------------------------------------------------------------------
# Dependency declaration
#---------------------------------------------------------------------------------
# Linker script for overriding libnx thread functions
nx_sys_thread_ld_override = meson.current_source_dir() / 'sys_thread_override.ld'

nx_sys_thread_dep = declare_dependency(
    sources : nx_sys_thread_tgt,
    link_with : nx_sys_thread_reent_lib,
    dependencies : deps,
)
