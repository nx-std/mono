project('nx-std', version : '0.1.0', meson_version : '>= 1.7.0')

cargo = find_program('cargo', required : true)

#---------------------------------------------------------------------------------
# Dependencies
#---------------------------------------------------------------------------------
deps = []
deps_override_link_args = []
deps_cargo_features = []
# Environment for this crate's cargo invocation. A dependency whose build needs
# more than features (today only `nx-rt-nso`, which takes its applet identity as
# a cfg value) contributes its own entries below.
deps_cargo_env = {}

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

# nx-fsdev
# The filesystem device sits on the descriptor table and the fsp-srv client, so both must be on
# for its overrides to have anything to dispatch through.
if get_option('use_nx_fsdev').enabled()
    if not get_option('use_nx_sys_fd').enabled()
        error('use_nx_fsdev requires -Duse_nx_sys_fd=enabled')
    endif
    if not get_option('use_nx_service_fs').enabled()
        error('use_nx_fsdev requires -Duse_nx_service_fs=enabled')
    endif

    nx_fsdev_proj = subproject('nx-fsdev')

    deps += nx_fsdev_proj.get_variable('nx_fsdev_dep')
    deps_override_link_args += ['-T', nx_fsdev_proj.get_variable('nx_fsdev_ld_override')]

    debug('fsdev feature: enabled')
    deps_cargo_features += ['fsdev']
endif

# nx-romfs
# The read-only device sits on the descriptor table and reads its bytes through the fsp-srv client,
# so both must be on for its overrides to have anything to dispatch through. `romfsMountSelf` is not
# in this crate's override surface: it is claimed per output kind by whichever `nx-rt-*` entry crate
# is in the link.
if get_option('use_nx_romfs').enabled()
    if not get_option('use_nx_sys_fd').enabled()
        error('use_nx_romfs requires -Duse_nx_sys_fd=enabled')
    endif
    if not get_option('use_nx_service_fs').enabled()
        error('use_nx_romfs requires -Duse_nx_service_fs=enabled')
    endif
    if not get_option('use_nx_fsdev').enabled()
        error('use_nx_romfs requires -Duse_nx_fsdev=enabled')
    endif

    nx_romfs_proj = subproject('nx-romfs')

    deps += nx_romfs_proj.get_variable('nx_romfs_dep')
    deps_override_link_args += ['-T', nx_romfs_proj.get_variable('nx_romfs_ld_override')]

    debug('romfs feature: enabled')
    deps_cargo_features += ['romfs']
endif

# nx-rand
if get_option('use_nx_rand').enabled()
    nx_rand_proj = subproject('nx-rand')

    deps += nx_rand_proj.get_variable('nx_rand_dep')
    deps_override_link_args += ['-T', nx_rand_proj.get_variable('nx_rand_ld_override')]

    debug('rand feature: enabled')
    deps_cargo_features += ['rand']
endif

# nx-std::env — a module of this crate, over the platform crates below it
if get_option('use_nx_std_env').enabled()
    nx_sys_args_proj = subproject('nx-sys-args')
    nx_sys_env_proj = subproject('nx-sys-env')
    nx_std_path_proj = subproject('nx-std-path')

    deps += nx_sys_args_proj.get_variable('nx_sys_args_dep')
    deps += nx_sys_env_proj.get_variable('nx_sys_env_dep')
    deps += nx_std_path_proj.get_variable('nx_std_path_dep')

    debug('env feature: enabled')
    deps_cargo_features += ['env']
endif

# nx-std-sync
if get_option('use_nx_std_sync').enabled()
    nx_std_sync_proj = subproject('nx-std-sync')

    deps += nx_std_sync_proj.get_variable('nx_std_sync_dep')

    debug('sync feature: enabled')
    deps_cargo_features += ['sync']
endif

# nx-std::thread — a module of this crate, over the platform crates below it
if get_option('use_nx_std_thread').enabled()
    nx_svc_thread_proj = subproject('nx-svc')
    nx_sys_thread_proj = subproject('nx-sys-thread')

    deps += nx_svc_thread_proj.get_variable('nx_svc_dep')
    deps += nx_sys_thread_proj.get_variable('nx_sys_thread_dep')

    debug('thread feature: enabled')
    deps_cargo_features += ['thread']
endif

# nx-svc
if get_option('use_nx_svc').enabled()
    nx_svc_proj = subproject('nx-svc')

    deps += nx_svc_proj.get_variable('nx_svc_dep')
    deps_override_link_args += ['-T', nx_svc_proj.get_variable('nx_svc_ld_override')]

    debug('svc feature: enabled')
    deps_cargo_features += ['svc']
endif

# nx-sys-env
if get_option('use_nx_sys_env').enabled()
    nx_sys_env_proj = subproject('nx-sys-env')

    deps += nx_sys_env_proj.get_variable('nx_sys_env_dep')
    deps_override_link_args += ['-T', nx_sys_env_proj.get_variable('nx_sys_env_ld_override')]

    debug('sys-env feature: enabled')
    deps_cargo_features += ['sys-env']
endif

# nx-sys-fd
if get_option('use_nx_sys_fd').enabled()
    nx_sys_fd_proj = subproject('nx-sys-fd')

    deps += nx_sys_fd_proj.get_variable('nx_sys_fd_dep')
    deps_override_link_args += ['-T', nx_sys_fd_proj.get_variable('nx_sys_fd_ld_override')]

    debug('sys-fd feature: enabled')
    deps_cargo_features += ['sys-fd']
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

# nx-sys-thread
if get_option('use_nx_sys_thread').enabled()
    nx_sys_thread_proj = subproject('nx-sys-thread')

    deps += nx_sys_thread_proj.get_variable('nx_sys_thread_dep')
    deps_override_link_args += ['-T', nx_sys_thread_proj.get_variable('nx_sys_thread_ld_override')]

    debug('sys-thread feature: enabled')
    deps_cargo_features += ['sys-thread']
endif

# nx-sys-mem
if get_option('use_nx_sys_mem').enabled()
    nx_sys_mem_proj = subproject('nx-sys-mem')
    nx_sys_virtmem_proj = subproject('nx-sys-virtmem')

    deps += nx_sys_mem_proj.get_variable('nx_sys_mem_dep')
    deps_override_link_args += ['-T', nx_sys_mem_proj.get_variable('nx_sys_mem_ld_override')]
    # The heap-free `virtmem` substrate lives in `nx-sys-virtmem`; it rides
    # along with `use_nx_sys_mem` since `nx-sys-mem` depends on it and the
    # `sys-mem` cargo feature pulls it in (no independent toggle).
    deps_override_link_args += ['-T', nx_sys_virtmem_proj.get_variable('nx_sys_virtmem_ld_override')]

    debug('sys-mem feature: enabled')
    deps_cargo_features += ['sys-mem']
endif

# nx-time
if get_option('use_nx_time').enabled()
    nx_time_proj = subproject('nx-time')

    deps += nx_time_proj.get_variable('nx_time_dep')
    deps_override_link_args += ['-T', nx_time_proj.get_variable('nx_time_ld_override')]

    debug('time feature: enabled')
    deps_cargo_features += ['time']
endif

# nx-rt-core + the nx_rt_kind entry crate
#
# The runtime is split across two crates: the kind-agnostic `nx-rt-core` (heap
# init, env state, HOS version, Service Manager) and one entry crate for the
# output kind being built. `nx_rt_kind` picks that entry crate: `nro` builds
# the homebrew-loader runtime (`nx-rt-hbapp`: loader-config parser, argv, applet
# dispatch), `nso` the `pm`-launched process runtime (`nx-rt-nso`: no loader
# block, `__argdata__` argv, build-time applet identity). Only one may be
# linked, since both bind the same `libnx` entry points (`envSetup`,
# `argvSetup`, `appletInitialize`); the `combo` resolves to a single choice,
# so the selection cannot be ambiguous. `nx-rt-core`'s `rt_core_libnx_core.ld`
# owns the shared `libnx` symbols and is linked first so the foundation
# overrides precede the kind-specific ones.
if get_option('use_nx_rt').enabled()
    nx_rt_core_proj = subproject('nx-rt-core')

    deps += nx_rt_core_proj.get_variable('nx_rt_core_dep')

    # `nx-rt-core`'s fragments are linked first so the foundation overrides
    # precede the kind-specific ones. The applet-accessor fragment is included
    # in `nx_rt_core_ld_overrides` only when `use_nx_service_applet` is enabled
    # (mirroring the `rt_<kind>_libnx_service_applet.ld` fragment in the entry
    # crate's list).
    foreach script : nx_rt_core_proj.get_variable('nx_rt_core_ld_overrides')
        deps_override_link_args += ['-T', script]
    endforeach

    if get_option('nx_rt_kind') == 'nso'
        nx_rt_nso_proj = subproject('nx-rt-nso')

        deps += nx_rt_nso_proj.get_variable('nx_rt_nso_dep')
        foreach script : nx_rt_nso_proj.get_variable('nx_rt_nso_ld_overrides')
            deps_override_link_args += ['-T', script]
        endforeach

        # The applet identity reaches `nx-rt-nso` as a cfg value, so this
        # crate's own cargo invocation has to carry it too; features alone do
        # not select it.
        deps_cargo_env += nx_rt_nso_proj.get_variable('nx_rt_nso_cargo_env')

        debug('rt-nso feature: enabled')
        deps_cargo_features += ['rt-nso']
    else
        nx_rt_hbapp_proj = subproject('nx-rt-hbapp')

        deps += nx_rt_hbapp_proj.get_variable('nx_rt_hbapp_dep')
        foreach script : nx_rt_hbapp_proj.get_variable('nx_rt_hbapp_ld_overrides')
            deps_override_link_args += ['-T', script]
        endforeach

        debug('rt-hbapp feature: enabled')
        deps_cargo_features += ['rt-hbapp']
    endif

    debug('rt feature: enabled')
    deps_cargo_features += ['rt']

    # The `rustc`-link pipeline takes its `_start` from the entry crate rather
    # than from libnx's `switch_crt0.s`. The feature has to be selected here,
    # on the staticlib that is actually linked: setting it on the entry crate's
    # own cargo invocation builds an rlib nothing consumes, and the `_start` it
    # emits never reaches the binary.
    if get_option('link_pipeline') == 'rustc'
        debug('rt-link feature: enabled')
        deps_cargo_features += ['rt-link']
    endif
endif

# nx-alloc requires the nx-rt runtime for heap initialization
#
# `nx-alloc` replaces the global allocator but never initializes its heap —
# that is wired by `nx-rt-core`'s `__libnx_initheap` override, which adopts the
# homebrew loader's heap region. With `use_nx_alloc` enabled but `use_nx_rt`
# disabled, the heap is never set up for a loader-hosted process and the build
# faults at startup on its first allocation.
if get_option('use_nx_alloc').enabled() and not get_option('use_nx_rt').enabled()
    warning(
        'use_nx_alloc is enabled without use_nx_rt: the nx-alloc heap is left '
        + 'uninitialized under a homebrew loader (hbloader) and the build will '
        + 'fault at startup on its first allocation. Enable use_nx_rt.',
    )
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

# nx-service-applet-album
# No `subproject(...)` entry, mirroring the absent Cargo dependency: the album
# applet's FFI lives in `nx-rt-hbapp`, which pulls the crate in and builds it.
if get_option('use_nx_service_applet_album').enabled()
    debug('service-applet-album feature: enabled')
    deps_cargo_features += ['service-applet-album']
endif

# nx-service-applet-err
# No `subproject(...)` entry, mirroring the absent Cargo dependency: the error
# applet's FFI lives in `nx-rt-hbapp`, which pulls the crate in and builds it.
if get_option('use_nx_service_applet_err').enabled()
    debug('service-applet-err feature: enabled')
    deps_cargo_features += ['service-applet-err']
endif

# nx-service-applet-friends
# No `subproject(...)` entry, mirroring the absent Cargo dependency: the MyPage
# applet's FFI lives in `nx-rt-hbapp`, which pulls the crate in and builds it.
if get_option('use_nx_service_applet_friends').enabled()
    debug('service-applet-friends feature: enabled')
    deps_cargo_features += ['service-applet-friends']
endif

# nx-service-applet-hid
# No `subproject(...)` entry, mirroring the absent Cargo dependency: the
# controller applet's FFI lives in `nx-rt-hbapp`, which pulls the crate in and
# builds it.
if get_option('use_nx_service_applet_hid').enabled()
    debug('service-applet-hid feature: enabled')
    deps_cargo_features += ['service-applet-hid']
endif

# nx-service-applet-mii
# No `subproject(...)` entry, mirroring the absent Cargo dependency: the Mii
# editor applet's FFI lives in `nx-rt-hbapp`, which pulls the crate in and builds it.
if get_option('use_nx_service_applet_mii').enabled()
    debug('service-applet-mii feature: enabled')
    deps_cargo_features += ['service-applet-mii']
endif

# nx-service-applet-nfp
# No `subproject(...)` entry, mirroring the absent Cargo dependency: the cabinet
# applet's FFI lives in `nx-rt-hbapp`, which pulls the crate in and builds it.
if get_option('use_nx_service_applet_nfp').enabled()
    debug('service-applet-nfp feature: enabled')
    deps_cargo_features += ['service-applet-nfp']
endif

# nx-service-applet-pctlauth
# No `subproject(...)` entry, mirroring the absent Cargo dependency: the auth
# applet's FFI lives in `nx-rt-hbapp`, which pulls the crate in and builds it.
if get_option('use_nx_service_applet_pctlauth').enabled()
    debug('service-applet-pctlauth feature: enabled')
    deps_cargo_features += ['service-applet-pctlauth']
endif

# nx-service-applet-psel
# No `subproject(...)` entry, mirroring the absent Cargo dependency: the
# player-select applet's FFI lives in `nx-rt-hbapp`, which pulls the crate in
# and builds it.
if get_option('use_nx_service_applet_psel').enabled()
    debug('service-applet-psel feature: enabled')
    deps_cargo_features += ['service-applet-psel']
endif

# nx-service-fs
if get_option('use_nx_service_fs').enabled()
    nx_service_fs_proj = subproject('nx-service-fs')

    deps += nx_service_fs_proj.get_variable('nx_service_fs_dep')

    debug('service-fs feature: enabled')
    deps_cargo_features += ['service-fs']
endif

# nx-service-hid
if get_option('use_nx_service_hid').enabled()
    nx_service_hid_proj = subproject('nx-service-hid')

    deps += nx_service_hid_proj.get_variable('nx_service_hid_dep')

    debug('service-hid feature: enabled')
    deps_cargo_features += ['service-hid']
endif

# nx-service-nv
if get_option('use_nx_service_nv').enabled()
    nx_service_nv_proj = subproject('nx-service-nv')

    deps += nx_service_nv_proj.get_variable('nx_service_nv_dep')

    debug('service-nv feature: enabled')
    deps_cargo_features += ['service-nv']
endif

# nx-service-set
if get_option('use_nx_service_set').enabled()
    nx_service_set_proj = subproject('nx-service-set')

    deps += nx_service_set_proj.get_variable('nx_service_set_dep')

    debug('service-set feature: enabled')
    deps_cargo_features += ['service-set']
endif

# nx-service-sfdnsres
if get_option('use_nx_service_sfdnsres').enabled()
    nx_service_sfdnsres_proj = subproject('nx-service-sfdnsres')

    deps += nx_service_sfdnsres_proj.get_variable('nx_service_sfdnsres_dep')

    debug('service-sfdnsres feature: enabled')
    deps_cargo_features += ['service-sfdnsres']
endif

# nx-service-sm
if get_option('use_nx_service_sm').enabled()
    nx_service_sm_proj = subproject('nx-service-sm')

    deps += nx_service_sm_proj.get_variable('nx_service_sm_dep')

    debug('service-sm feature: enabled')
    deps_cargo_features += ['service-sm']
endif

# nx-service-time
if get_option('use_nx_service_time').enabled()
    nx_service_time_proj = subproject('nx-service-time')

    deps += nx_service_time_proj.get_variable('nx_service_time_dep')

    debug('service-time feature: enabled')
    deps_cargo_features += ['service-time']
endif

# nx-service-vi
if get_option('use_nx_service_vi').enabled()
    nx_service_vi_proj = subproject('nx-service-vi')

    deps += nx_service_vi_proj.get_variable('nx_service_vi_dep')

    debug('service-vi feature: enabled')
    deps_cargo_features += ['service-vi']
endif

# nx-display
# The `display` cargo feature pulls in `service-vi`, so enabling
# `use_nx_display` implicitly enables `service-vi` via Cargo's feature graph.
# `nx-display`'s own meson.build already calls `subproject('nx-service-vi')`,
# so meson's build ordering also picks it up.
if get_option('use_nx_display').enabled()
    nx_display_proj = subproject('nx-display')

    deps += nx_display_proj.get_variable('nx_display_dep')

    debug('display feature: enabled')
    deps_cargo_features += ['display']
endif

# nx-pm
# `nx-pm` depends on `nx-service-pm` internally; enabling `use_nx_pm` pulls
# the service crate in transitively via Cargo and Meson's subproject graph.
if get_option('use_nx_pm').enabled()
    nx_pm_proj = subproject('nx-pm')

    deps += nx_pm_proj.get_variable('nx_pm_dep')
    deps_override_link_args += ['-T', nx_pm_proj.get_variable('nx_pm_ld_override')]

    debug('pm feature: enabled')
    deps_cargo_features += ['pm']
endif

# nx-wlaninf
# `nx-wlaninf` depends on `nx-service-wlaninf` internally; enabling
# `use_nx_wlaninf` pulls the service crate in transitively via Cargo and
# Meson's subproject graph.
if get_option('use_nx_wlaninf').enabled()
    nx_wlaninf_proj = subproject('nx-wlaninf')

    deps += nx_wlaninf_proj.get_variable('nx_wlaninf_dep')
    deps_override_link_args += ['-T', nx_wlaninf_proj.get_variable('nx_wlaninf_ld_override')]

    debug('wlaninf feature: enabled')
    deps_cargo_features += ['wlaninf']
endif

# nx-net
# `nx-net` depends on `nx-service-sfdnsres`, `nx-service-sm`, and the global
# allocator internally; enabling `use_nx_net` pulls those crates in
# transitively via Cargo and Meson's subproject graph.
# The five resolver symbols that reach `sfdnsres` are overridden by
# `nx-rt-core`, not here: acquiring that service needs the process's
# service-manager session, and a process gets one, which the runtime holds.
# That fragment is appended by `nx-rt-core`'s own override list, gated on this
# option.
if get_option('use_nx_net').enabled()
    if not get_option('use_nx_rt').enabled()
        error(
            'use_nx_net=enabled requires -Duse_nx_rt=enabled (currently @0@): the runtime holds the service-manager session the resolver is acquired over, and without it the five service-touching symbols have no override'.format(
                get_option('use_nx_rt'),
            ),
        )
    endif

    nx_net_proj = subproject('nx-net')

    deps += nx_net_proj.get_variable('nx_net_dep')
    deps_override_link_args += ['-T', nx_net_proj.get_variable('nx_net_ld_override')]

    debug('net feature: enabled')
    deps_cargo_features += ['net']
endif

# nx-sys-net
# The socket driver. Depends on `nx-service-bsd` and `nx-sys-fd` internally;
# enabling `use_nx_sys_net` pulls both in transitively via Cargo and Meson's
# subproject graph.
#
# It requires `use_nx_sys_fd`: its sockets are held in that crate's descriptor
# table, and with libsysbase's own table still live the `read`/`write`/`close`
# a C caller makes would resolve there instead, against a table that has never
# heard of the socket device.
if get_option('use_nx_sys_net').enabled()
    if not get_option('use_nx_sys_fd').enabled()
        error(
            'use_nx_sys_net=enabled requires -Duse_nx_sys_fd=enabled (currently @0@): socket descriptors live in the Rust descriptor table, and read/write/close on one only reach the socket device through it'.format(
                get_option('use_nx_sys_fd'),
            ),
        )
    endif

    # `socketInitialize` is overridden by `nx-rt-core`, because the interface revision it
    # declares follows the running firmware and only the runtime holds that. Without the
    # runtime, libnx's own `socketInitialize` survives and brings up libnx's BSD client and
    # devoptab instead of this driver, leaving two socket layers live over separate state.
    if not get_option('use_nx_rt').enabled()
        error(
            'use_nx_sys_net=enabled requires -Duse_nx_rt=enabled (currently @0@): the runtime owns socketInitialize, and without it libnx brings up its own socket driver alongside this one'.format(
                get_option('use_nx_rt'),
            ),
        )
    endif

    # `socketInitialize` is overridden by `nx-rt-core`, not here: its interface
    # revision follows the running firmware, which only the runtime can read. That
    # fragment is appended by `nx-rt-core`'s own override list, gated on this option.
    nx_sys_net_proj = subproject('nx-sys-net')

    deps += nx_sys_net_proj.get_variable('nx_sys_net_dep')
    deps_override_link_args += ['-T', nx_sys_net_proj.get_variable('nx_sys_net_ld_override')]

    debug('sys-net feature: enabled')
    deps_cargo_features += ['sys-net']
endif

#---------------------------------------------------------------------------------
# Configuration constraints
#---------------------------------------------------------------------------------
# Any Rust feature above reaches IPC, and IPC reads a `#[thread_local]` that LLVM
# resolves against `tpidr_el0`. Horizon does not maintain that register: it is
# written by `nx_sys_thread_tls::init_thread_vars`, and only Rust calls that -
# `nx-rt-core` for the main thread, `nx-sys-thread` for spawned ones.
#
# libnx's own `newlibSetup`/`threadCreate` set the soft thread pointer in
# `ThreadVars.tls_ptr`, which is what `__aarch64_read_tp` reads, and leave
# `tpidr_el0` alone. A thread they started therefore has no valid base, and a
# Rust thread-local read on it resolves against a stale one - returning another
# thread's memory rather than faulting, which is why this is refused at
# configure time instead of left to be found at runtime.
if deps_cargo_features.length() > 0
    if not get_option('use_nx_rt').enabled()
        error(
            'nx-std with Rust features enabled requires -Duse_nx_rt=enabled (currently @0@): otherwise libnx starts the main thread and leaves tpidr_el0 unset, so Rust thread-locals resolve against a stale base'.format(
                get_option('use_nx_rt'),
            ),
        )
    endif

    if not get_option('use_nx_sys_thread').enabled()
        error(
            'nx-std with Rust features enabled requires -Duse_nx_sys_thread=enabled (currently @0@): otherwise libnx starts spawned threads and leaves tpidr_el0 unset, so Rust thread-locals resolve against a stale base'.format(
                get_option('use_nx_sys_thread'),
            ),
        )
    endif
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
    env : deps_cargo_env,
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
