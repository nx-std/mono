project('nx-std', version : '0.1.0', meson_version : '>= 1.7.0')

cargo = find_program('cargo', required : true)

#---------------------------------------------------------------------------------
# Features
#---------------------------------------------------------------------------------
# The `use_nx_*` feature set, resolved the way Cargo resolves `[features]`: a
# feature is asked for by name, and asking for it also asks for what it depends
# on.
#
# "Feature" is the word at both ends of this: each entry is a Meson `feature`
# option, and enabling it adds the matching Cargo feature to this crate's build
# and links a `*_override.ld` that aliases a family of `libnx` symbols onto the
# Rust implementation behind it. `use_nx_service_fs` is the `service-fs` Cargo
# feature plus `rt_nro_libnx_service_fs.ld`, and so on down the table.
#
# This file is the one place the set is resolved. It assembles the staticlib,
# so every consequence of a feature - the Cargo feature that compiles the
# symbols and the `*_override.ld` fragment that aliases them - is selected here
# by the same answer, and the two cannot disagree. The crates below export
# their fragments unconditionally and decide nothing; the binary subprojects
# above gate on single bottom-of-graph features through the `use_nx` master
# switch alone, which needs no table.

# Each feature and the features it cannot work without.
#
# Aliasing half a family does not degrade: the C side keeps calling the symbols
# it always called, some of them now land in Rust, and the two halves disagree
# about state neither can see. `fsdev` without `service_fs` is the shape of it -
# the Rust device is aliased over `fsdevMountSdmc`, `fsInitialize` stays libnx's,
# so no Rust session is ever installed and every mount answers `NOT_FOUND` on a
# build that linked cleanly.
#
# The entries are direct edges only; the resolution below takes the closure, so
# `fsdev` reaches `rt` through `service_fs` without saying so twice.
#
# Two facts drive most of these:
#
# - A `service_*` feature's `<svc>Initialize` entry point lives in `nx-rt-nro`
#   or `nx-rt-core`, and those crates are only linked under `rt`. Without it the
#   service's aliases are never emitted, so the feature has no effect at all -
#   which is a configuration mistake worth reporting rather than ignoring.
# - `nx-alloc` and `nx-fsdev` are linked by this crate directly, so their aliases
#   land whatever the runtime does. These are the ones that break rather than
#   go quiet.
nx_features_deps = {
    'alloc' : ['rt'], # heap is initialized by nx-rt-core's `__libnx_initheap`
    'display' : ['service_vi'], # the `display` cargo feature pulls in `service-vi`
    'fsdev' : ['sys_fd', 'service_fs'], # registers with the fd table, dispatches through fsp-srv
    'pm' : [],
    'rand' : [],
    'rt' : [],
    'service_apm' : ['rt'],
    'service_applet' : ['rt'],
    'service_applet_err' : ['service_applet'],
    'service_fs' : ['rt'],
    'service_hid' : ['service_applet'],
    'service_nv' : ['service_applet'],
    'service_set' : ['rt'],
    'service_sfdnsres' : [],
    'service_sm' : ['rt'],
    'service_time' : ['rt', 'time'], # the clock anchor and the syscalls reading it must agree
    'service_vi' : ['rt'],
    'sf' : [],
    'std_sync' : [],
    'svc' : [],
    'sys_fd' : [],
    'sys_mem' : [],
    'sys_sync' : [],
    'sys_thread' : [],
    'sys_thread_tls' : [],
    'time' : [],
    'wlaninf' : [],
}

# Which features can be wired at all, which the choice of `libnx` archive
# decides.
#
# The prebuilt devkitPro archive brings its own runtime and services; a feature
# that would alias over them has nothing to attach to, so it is unavailable
# rather than merely disabled. The distinction matters below: a dependency edge
# pointing at an unavailable feature is satisfied by the C archive instead of
# going unmet, which is why `alloc` still works against the prebuilt libnx
# despite depending on `rt` against the one built from source.
nx_features_dkp_supported = [
    'alloc',
    'rand',
    'std_sync',
    'svc',
    'sys_mem',
    'sys_sync',
    'time',
]

nx_features_available = {}
foreach name, _ : nx_features_deps
    nx_features_available += {name : not get_option('use_libnx_dkp').enabled() or name in nx_features_dkp_supported}
endforeach

# `use_nx` is this table's `default` feature set: it decides every feature left
# on `auto`. A feature named explicitly overrides it.
use_nx = get_option('use_nx')

# What was asked for, kept apart from what it resolves to.
#
# The distinction is the whole of the resolution: a feature the master switch
# enabled may be disabled again by a missing dependency, while one enabled by
# hand may not - that combination is an error instead. Collapse the two and a
# dependency edge can no longer refuse anything, because after the master switch
# every `auto` looks exactly like an explicit `enabled`.
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

nx_features = {}
foreach name, asked : nx_features_asked
    if asked.enabled() and not nx_features_available[name]
        error(
            'use_nx_@0@ cannot be enabled against the prebuilt devkitPro libnx: that archive '.format(name)
            + 'brings its own implementation, so there is nothing for the override to alias '
            + 'over. Drop use_libnx_dkp to build libnx from source.',
        )
    endif
    nx_features += {name : nx_features_available[name] and asked.enable_auto_if(use_nx.enabled()).enabled()}
endforeach

# Apply the dependency edges in both directions:
#
# - pull up, so an enabled feature brings in what it depends on and a single
#   `-Duse_nx_fsdev=enabled` produces a build that works rather than one that
#   needs three more flags to be told about;
# - push down, so a feature whose dependency is disabled is disabled again
#   instead of leaving the family half-aliased.
#
# Neither moves a feature the user named: pull-up only fills an `auto`, and
# push-down only retracts one. Two explicit choices that contradict each other
# are reported below rather than silently reconciled.
#
# Both skip an edge pointing at an unavailable feature, which is the C archive's
# half of the deal: under the prebuilt libnx that dependency is already met by
# the archive, so `alloc` neither drags `rt` in nor is retracted for its absence.
#
# The graph is a DAG, so repeating the pass once per entry reaches a fixed point
# whatever order the table is written in. Deriving an order from the table
# instead would make the table's order load-bearing.
foreach _ : range(nx_features_deps.keys().length())
    foreach name, deps : nx_features_deps
        foreach dep : deps
            if nx_features[name] and nx_features_available[dep] and nx_features_asked[dep].auto()
                nx_features += {dep : true}
            endif
            if not nx_features[dep] and nx_features_available[dep] and nx_features_asked[name].auto()
                nx_features += {name : false}
            endif
        endforeach
    endforeach
endforeach

foreach name, deps : nx_features_deps
    foreach dep : deps
        if nx_features_asked[name].enabled() and not nx_features[dep] and nx_features_available[dep]
            error(
                'use_nx_@0@ depends on use_nx_@1@, which is disabled: the aliases would be emitted '.format(name, dep)
                + 'for one half of the pair and not the other, which links cleanly and fails at '
                + 'run time. Enable use_nx_@1@ or disable use_nx_@0@.'.format(name, dep),
            )
        endif
    endforeach
endforeach

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
if nx_features['alloc']
    nx_alloc_proj = subproject('nx-alloc')

    deps += nx_alloc_proj.get_variable('nx_alloc_dep')
    deps_override_link_args += ['-T', nx_alloc_proj.get_variable('nx_alloc_ld_override')]

    debug('alloc feature: enabled')
    deps_cargo_features += ['alloc']
endif

# nx-fsdev
# What this device needs to have anything to dispatch through - the descriptor
# table and the fsp-srv client - is declared as a dependency edge in the feature
# table above, which is where the combination is refused. Reaching this block
# means it holds.
if nx_features['fsdev']
    nx_fsdev_proj = subproject('nx-fsdev')

    deps += nx_fsdev_proj.get_variable('nx_fsdev_dep')
    deps_override_link_args += ['-T', nx_fsdev_proj.get_variable('nx_fsdev_ld_override')]

    debug('fsdev feature: enabled')
    deps_cargo_features += ['fsdev']
endif

# nx-rand
if nx_features['rand']
    nx_rand_proj = subproject('nx-rand')

    deps += nx_rand_proj.get_variable('nx_rand_dep')
    deps_override_link_args += ['-T', nx_rand_proj.get_variable('nx_rand_ld_override')]

    debug('rand feature: enabled')
    deps_cargo_features += ['rand']
endif

# nx-std-sync
if nx_features['std_sync']
    nx_std_sync_proj = subproject('nx-std-sync')

    deps += nx_std_sync_proj.get_variable('nx_std_sync_dep')

    debug('sync feature: enabled')
    deps_cargo_features += ['sync']
endif

# nx-svc
if nx_features['svc']
    nx_svc_proj = subproject('nx-svc')

    deps += nx_svc_proj.get_variable('nx_svc_dep')
    deps_override_link_args += ['-T', nx_svc_proj.get_variable('nx_svc_ld_override')]

    debug('svc feature: enabled')
    deps_cargo_features += ['svc']
endif

# nx-sys-fd
if nx_features['sys_fd']
    nx_sys_fd_proj = subproject('nx-sys-fd')

    deps += nx_sys_fd_proj.get_variable('nx_sys_fd_dep')
    deps_override_link_args += ['-T', nx_sys_fd_proj.get_variable('nx_sys_fd_ld_override')]

    debug('sys-fd feature: enabled')
    deps_cargo_features += ['sys-fd']
endif

# nx-sys-sync
if nx_features['sys_sync']
    nx_sys_sync_proj = subproject('nx-sys-sync')

    deps += nx_sys_sync_proj.get_variable('nx_sys_sync_dep')
    deps_override_link_args += ['-T', nx_sys_sync_proj.get_variable('nx_sys_sync_ld_override')]

    debug('sys-sync feature: enabled')
    deps_cargo_features += ['sys-sync']
endif

# nx-sys-thread-tls
if nx_features['sys_thread_tls']
    nx_sys_thread_tls_proj = subproject('nx-sys-thread-tls')

    deps += nx_sys_thread_tls_proj.get_variable('nx_sys_thread_tls_dep')
    deps_override_link_args += ['-T', nx_sys_thread_tls_proj.get_variable('nx_sys_thread_tls_ld_override')]

    debug('sys-thread-tls feature: enabled')
    deps_cargo_features += ['sys-thread-tls']
endif

# nx-sys-thread
if nx_features['sys_thread']
    nx_sys_thread_proj = subproject('nx-sys-thread')

    deps += nx_sys_thread_proj.get_variable('nx_sys_thread_dep')
    deps_override_link_args += ['-T', nx_sys_thread_proj.get_variable('nx_sys_thread_ld_override')]

    debug('sys-thread feature: enabled')
    deps_cargo_features += ['sys-thread']
endif

# nx-sys-mem
if nx_features['sys_mem']
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
if nx_features['time']
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
# the homebrew-loader runtime (`nx-rt-nro`: loader-config parser, argv, applet
# dispatch), `nso` the `pm`-launched process runtime (`nx-rt-nso`: no loader
# block, `__argdata__` argv, build-time applet identity). Only one may be
# linked, since both bind the same `libnx` entry points (`envSetup`,
# `argvSetup`, `appletInitialize`); the `combo` resolves to a single choice,
# so the selection cannot be ambiguous. `nx-rt-core`'s `rt_core_libnx_core.ld`
# owns the shared `libnx` symbols and is linked first so the foundation
# overrides precede the kind-specific ones.
if nx_features['rt']
    nx_rt_core_proj = subproject('nx-rt-core')

    deps += nx_rt_core_proj.get_variable('nx_rt_core_dep')

    # The runtime crates export every fragment unconditionally and decide
    # nothing; which fragments are linked is decided here, by the same resolved
    # answer that selects the Cargo features compiling the symbols they bind.
    #
    # `nx-rt-core`'s fragments are linked first so the foundation overrides
    # precede the kind-specific ones. The applet-accessor fragment binds
    # symbols that exist only under the `service-applet` Cargo feature
    # (mirroring the `rt_<kind>_libnx_service_applet.ld` fragment linked in
    # the entry-crate branch below).
    deps_override_link_args += ['-T', nx_rt_core_proj.get_variable('nx_rt_core_ld_override')]
    if nx_features['service_applet']
        deps_override_link_args += ['-T', nx_rt_core_proj.get_variable('nx_rt_core_service_applet_ld_override')]
    endif

    if get_option('nx_rt_kind') == 'nso'
        nx_rt_nso_proj = subproject('nx-rt-nso')

        deps += nx_rt_nso_proj.get_variable('nx_rt_nso_dep')
        # Both NSO fragments are genuinely unconditional - every NSO brings up
        # env/argv and carries an applet-type value - so the list stays a list.
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
        nx_rt_nro_proj = subproject('nx-rt-nro')

        deps += nx_rt_nro_proj.get_variable('nx_rt_nro_dep')
        deps_override_link_args += ['-T', nx_rt_nro_proj.get_variable('nx_rt_nro_ld_override')]
        if nx_features['service_apm']
            deps_override_link_args += ['-T', nx_rt_nro_proj.get_variable('nx_rt_nro_service_apm_ld_override')]
        endif
        if nx_features['service_applet']
            deps_override_link_args += ['-T', nx_rt_nro_proj.get_variable('nx_rt_nro_service_applet_ld_override')]
        endif
        if nx_features['service_applet_err']
            deps_override_link_args += ['-T', nx_rt_nro_proj.get_variable('nx_rt_nro_service_applet_err_ld_override')]
        endif
        if nx_features['service_fs']
            deps_override_link_args += ['-T', nx_rt_nro_proj.get_variable('nx_rt_nro_service_fs_ld_override')]
        endif
        if nx_features['service_hid']
            deps_override_link_args += ['-T', nx_rt_nro_proj.get_variable('nx_rt_nro_service_hid_ld_override')]
        endif
        if nx_features['service_nv']
            deps_override_link_args += ['-T', nx_rt_nro_proj.get_variable('nx_rt_nro_service_nv_ld_override')]
        endif
        if nx_features['service_set']
            deps_override_link_args += ['-T', nx_rt_nro_proj.get_variable('nx_rt_nro_service_set_ld_override')]
        endif
        if nx_features['service_time']
            deps_override_link_args += ['-T', nx_rt_nro_proj.get_variable('nx_rt_nro_service_time_ld_override')]
        endif
        if nx_features['service_vi']
            deps_override_link_args += ['-T', nx_rt_nro_proj.get_variable('nx_rt_nro_service_vi_ld_override')]
        endif

        debug('rt-nro feature: enabled')
        deps_cargo_features += ['rt-nro']
    endif

    debug('rt feature: enabled')
    deps_cargo_features += ['rt']
endif

# `nx-alloc` replaces the global allocator but never initializes its heap - that
# is wired by `nx-rt-core`'s `__libnx_initheap` override, which adopts the
# homebrew loader's heap region. That is why `alloc` depends on `rt` in the
# feature table above; the pair used to be a warning here, which a build could
# and did scroll past on its way to faulting at startup on the first allocation.

# nx-sf
if nx_features['sf']
    nx_sf_proj = subproject('nx-sf')

    deps += nx_sf_proj.get_variable('nx_sf_dep')
    deps_override_link_args += ['-T', nx_sf_proj.get_variable('nx_sf_ld_override')]

    debug('sf feature: enabled')
    deps_cargo_features += ['sf']
endif

# nx-service-apm
if nx_features['service_apm']
    nx_service_apm_proj = subproject('nx-service-apm')

    deps += nx_service_apm_proj.get_variable('nx_service_apm_dep')

    debug('service-apm feature: enabled')
    deps_cargo_features += ['service-apm']
endif

# nx-service-applet
if nx_features['service_applet']
    nx_service_applet_proj = subproject('nx-service-applet')

    deps += nx_service_applet_proj.get_variable('nx_service_applet_dep')

    debug('service-applet feature: enabled')
    deps_cargo_features += ['service-applet']
endif

# nx-service-applet-err
# No `subproject(...)` entry, mirroring the absent Cargo dependency: the error
# applet's FFI lives in `nx-rt-nro`, which pulls the crate in and builds it.
if nx_features['service_applet_err']
    debug('service-applet-err feature: enabled')
    deps_cargo_features += ['service-applet-err']
endif

# nx-service-fs
if nx_features['service_fs']
    nx_service_fs_proj = subproject('nx-service-fs')

    deps += nx_service_fs_proj.get_variable('nx_service_fs_dep')

    debug('service-fs feature: enabled')
    deps_cargo_features += ['service-fs']
endif

# nx-service-hid
if nx_features['service_hid']
    nx_service_hid_proj = subproject('nx-service-hid')

    deps += nx_service_hid_proj.get_variable('nx_service_hid_dep')

    debug('service-hid feature: enabled')
    deps_cargo_features += ['service-hid']
endif

# nx-service-nv
if nx_features['service_nv']
    nx_service_nv_proj = subproject('nx-service-nv')

    deps += nx_service_nv_proj.get_variable('nx_service_nv_dep')

    debug('service-nv feature: enabled')
    deps_cargo_features += ['service-nv']
endif

# nx-service-set
if nx_features['service_set']
    nx_service_set_proj = subproject('nx-service-set')

    deps += nx_service_set_proj.get_variable('nx_service_set_dep')

    debug('service-set feature: enabled')
    deps_cargo_features += ['service-set']
endif

# nx-service-sfdnsres
if nx_features['service_sfdnsres']
    nx_service_sfdnsres_proj = subproject('nx-service-sfdnsres')

    deps += nx_service_sfdnsres_proj.get_variable('nx_service_sfdnsres_dep')

    debug('service-sfdnsres feature: enabled')
    deps_cargo_features += ['service-sfdnsres']
endif

# nx-service-sm
if nx_features['service_sm']
    nx_service_sm_proj = subproject('nx-service-sm')

    deps += nx_service_sm_proj.get_variable('nx_service_sm_dep')

    debug('service-sm feature: enabled')
    deps_cargo_features += ['service-sm']
endif

# nx-service-time
if nx_features['service_time']
    nx_service_time_proj = subproject('nx-service-time')

    deps += nx_service_time_proj.get_variable('nx_service_time_dep')

    debug('service-time feature: enabled')
    deps_cargo_features += ['service-time']
endif

# nx-service-vi
if nx_features['service_vi']
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
if nx_features['display']
    nx_display_proj = subproject('nx-display')

    deps += nx_display_proj.get_variable('nx_display_dep')

    debug('display feature: enabled')
    deps_cargo_features += ['display']
endif

# nx-pm
# `nx-pm` depends on `nx-service-pm` internally; enabling `use_nx_pm` pulls
# the service crate in transitively via Cargo and Meson's subproject graph.
if nx_features['pm']
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
if nx_features['wlaninf']
    nx_wlaninf_proj = subproject('nx-wlaninf')

    deps += nx_wlaninf_proj.get_variable('nx_wlaninf_dep')
    deps_override_link_args += ['-T', nx_wlaninf_proj.get_variable('nx_wlaninf_ld_override')]

    debug('wlaninf feature: enabled')
    deps_cargo_features += ['wlaninf']
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
