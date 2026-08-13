project('nx-tests', 'c',
        version : '1.0.0',
        meson_version : '>= 1.7.0',
        default_options : [
            'buildtype=debugoptimized', # -g -O2
            'warning_level=1', # -Wall
        ])

# Project metadata
author = 'LNSD'
version = meson.project_version()
# NACP `version` field is fixed at 16 bytes; keep a separate short string for it.
nacp_version = version

# If this is not a release build, use the git tag as the version
if get_option('buildtype') != 'release'
    git_rev = run_command('git', 'describe', '--dirty', '--always', check : true).stdout().strip()
    version += '-@0@'.format(git_rev)
endif

#---------------------------------------------------------------------------------
# Dependencies
#---------------------------------------------------------------------------------
# libnx
if get_option('use_libnx_dkp').enabled()
    nx_proj = subproject('libnx-dkp')
else
    nx_proj = subproject('libnx')
endif

nx_dep = nx_proj.get_variable('nx_dep')
nx_switch_specs = nx_proj.get_variable('nx_switch_specs')
nx_default_icon = nx_proj.get_variable('nx_default_icon')

# nx-std-sync
# Tests link against the Rust oneshot FFI exported by nx-std-sync. That FFI is
# only emitted when `use_nx_std_sync` is explicitly enabled — `auto` resolves to
# off here because libnx itself does not pull the symbols in. Fail configuration
# early instead of letting the linker report missing `__nx_std_sync__*` symbols.
if not get_option('use_nx_std_sync').enabled()
    error('nx-tests requires -Duse_nx_std_sync=enabled (currently @0@)'.format(get_option('use_nx_std_sync')))
endif

nx_std_sync_proj = subproject('nx-std-sync')
nx_std_sync_dep = nx_std_sync_proj.get_variable('nx_std_sync_dep')

# nx-netloader
# The runner serves the netloader protocol through this crate. It stands on the
# Rust socket driver rather than on libnx's, so `use_nx_sys_net` has to be on:
# with it off the driver the crate talks to is never initialized and every call
# reports a socket that was never connected. Fail configuration early instead of
# letting the runner come up unable to listen.
if not get_option('use_nx_sys_net').enabled()
    error('nx-tests requires -Duse_nx_sys_net=enabled (currently @0@)'.format(get_option('use_nx_sys_net')))
endif

# The netloader writes what it receives through the Rust filesystem API, which reaches a device by
# resolving the path against the descriptor registry. A device registered from C is held there only
# as a name: it is dispatched by C through the devoptab table, so its Rust `Device` methods are the
# trait's defaults and opening a path on one reports the operation as unsupported. So the SD card
# has to be served by the Rust device, not libnx's.
if not get_option('use_nx_fsdev').enabled()
    error('nx-tests requires -Duse_nx_fsdev=enabled (currently @0@): the netloader opens files through the Rust filesystem device, and a device registered from C cannot serve one'.format(get_option('use_nx_fsdev')))
endif

nx_netloader_proj = subproject('nx-netloader')
nx_netloader_dep = nx_netloader_proj.get_variable('nx_netloader_dep')

#---------------------------------------------------------------------------------
# Build options
#---------------------------------------------------------------------------------
arch_opts = ['-march=armv8-a+crc+crypto', '-mtune=cortex-a57', '-mtp=soft', '-fPIE']
c_opts = ['-ffunction-sections'] + arch_opts + ['-D__SWITCH__', '-DVERSION="v@0@"'.format(version)]
cpp_opts = c_opts + ['-fno-rtti', '-fno-exceptions']
# The map file is named after the target, so the full link args are assembled
# per NRO below rather than shared here.
ld_base = ['-specs=@0@'.format(nx_switch_specs), '-g'] + arch_opts

#---------------------------------------------------------------------------------
# Bundling
#---------------------------------------------------------------------------------
fs = import('fs')
bundle_sh = find_program('bundle', required : true)
nm = find_program('nm', required : true)
find_program('elf2nro', required : true)
find_program('nacptool', required : true)

# NRO icon: a local icon.jpg wins, otherwise libnx's default.
if fs.is_file('icon.jpg')
    icon = 'icon.jpg'
elif fs.is_file(nx_default_icon)
    icon = nx_default_icon
else
    error('No icon file found')
endif

bundle_base = [
    bundle_sh,
    '--out-dir', '@OUTDIR@',
    '--input', '@INPUT@',
    '--output', '@OUTPUT0@',
    '--tmp-dir', '@PRIVATE_DIR@',
    '--icon', icon,
    '--author', author,
    '--version', nacp_version,
]

#---------------------------------------------------------------------------------
# Test runner NRO
#---------------------------------------------------------------------------------

# ── nx-tests ──────────────────────────────────────────────────────────────
# The runner, not a suite: it serves the netloader protocol so the host can push
# a test NRO onto the console at any moment, and hands what it receives to the
# process loader to run. Running a whole run therefore takes one launch by hand
# instead of one per binary.
#
# It links against libnx alone. Everything a suite needs, the runner would have
# to bring up in the same process and take down again before handing over, so it
# keeps its own dependencies to what serving the protocol costs.
name = 'nx-tests'
src = files(
    'source/rig.h',
    'source/runner/handback.h',
    'source/runner/ledger.h',
    'source/runner/ledger.c',
    'source/runner/main.c',
)
elf = executable(name + '.elf', src,
                 dependencies : [nx_dep, nx_netloader_dep],
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_base + ['-Wl,-Map,@0@/@1@.map'.format(meson.current_build_dir(), name)],
                 pie : true)
custom_target(name + '.lst',
              input : elf, output : name + '.lst',
              command : [nm, '-CSn', '@INPUT@'], capture : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

#---------------------------------------------------------------------------------
# Test NROs
#---------------------------------------------------------------------------------
# One NRO per area, which is the unit the runner hands over: an area that fails
# for a reason of its own — a missing SD card, a network that will not come up, a
# person who has to dismiss a dialog — takes down its own binary and no other's.

# ── nx-tests-rand ─────────────────────────────────────────────────────────
# Runs unattended. It links either way: with `use_nx_rand` off the calls resolve
# to libnx, which is the baseline to compare our implementation against.
name = 'nx-tests-rand'
src = files(
    'source/rig.h',
    'source/suites/handback.h',
    'source/suites/tap.h',
    'source/suites/tap.c',
    'source/suites/rand/suite.h',
    'source/suites/rand/test_0001_rand_get_fills_buffers_with_random_data.c',
    'source/suites/rand/test_0002_rand_get64_returns_different_values.c',
    'source/suites/rand/main.c',
)
elf = executable(name + '.elf', src,
                 dependencies : [nx_dep, nx_std_sync_dep],
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_base + ['-Wl,-Map,@0@/@1@.map'.format(meson.current_build_dir(), name)],
                 pie : true)
custom_target(name + '.lst',
              input : elf, output : name + '.lst',
              command : [nm, '-CSn', '@INPUT@'], capture : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── nx-tests-rt ───────────────────────────────────────────────────────────
# Runs unattended, and reads the command line it was started with: the cases
# here are about the working directory the runtime derives from it, so what they
# see depends on who launched the binary. It links either way: with `use_nx_rt`
# off the runtime is libnx's own, the baseline to compare against.
name = 'nx-tests-rt'
src = files(
    'source/rig.h',
    'source/suites/handback.h',
    'source/suites/tap.h',
    'source/suites/tap.c',
    'source/suites/rt/cwd/program.h',
    'source/suites/rt/cwd/suite.h',
    'source/suites/rt/cwd/test_0001_startup_cwd_is_the_program_directory.c',
    'source/suites/rt/cwd/test_0002_a_bare_path_resolves_next_to_the_program.c',
    'source/suites/rt/main.c',
)
elf = executable(name + '.elf', src,
                 dependencies : [nx_dep, nx_std_sync_dep],
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_base + ['-Wl,-Map,@0@/@1@.map'.format(meson.current_build_dir(), name)],
                 pie : true)
custom_target(name + '.lst',
              input : elf, output : name + '.lst',
              command : [nm, '-CSn', '@INPUT@'], capture : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── nx-tests-thread ───────────────────────────────────────────────────────
# Runs unattended. It links either way, but not to the same result: with
# `use_nx_sys_thread` off, `thrd_detach` is a libnx stub that returns an error,
# so the detach cases fail there by construction rather than by regression.
name = 'nx-tests-thread'
src = files(
    'source/rig.h',
    'source/suites/handback.h',
    'source/suites/tap.h',
    'source/suites/tap.c',
    'source/suites/thread/suite.h',
    'source/suites/thread/test_0001_detach_running_thread_self_reclaims.c',
    'source/suites/thread/test_0002_detach_after_exit_reclaims.c',
    'source/suites/thread/test_0003_thread_pointer_is_per_thread_and_preserved.c',
    'source/suites/thread/main.c',
)
elf = executable(name + '.elf', src,
                 dependencies : [nx_dep, nx_std_sync_dep],
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_base + ['-Wl,-Map,@0@/@1@.map'.format(meson.current_build_dir(), name)],
                 pie : true)
custom_target(name + '.lst',
              input : elf, output : name + '.lst',
              command : [nm, '-CSn', '@INPUT@'], capture : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── nx-tests-sync ─────────────────────────────────────────────────────────
# The synchronization primitives, kept in their own binary because they stand on
# the C ABI of a lock: a C caller allocates the object and a Rust implementation
# has to agree to the byte about what is inside it. A disagreement writes past
# the end of the caller's object rather than failing, so the damage surfaces
# somewhere else entirely — which is exactly what must not be allowed to land in
# the middle of an unrelated suite's results. It links either way: with
# `use_nx_sys_sync` off the primitives resolve to the C implementation, the
# baseline to compare against.
name = 'nx-tests-sync'
src = files(
    'source/rig.h',
    'source/suites/handback.h',
    'source/suites/tap.h',
    'source/suites/tap.c',
    'source/suites/sync/suite.h',
    'source/suites/sync/mutex/suite.h',
    'source/suites/sync/mutex/test_0001_mutex_lock_unlock_single_thread.c',
    'source/suites/sync/mutex/test_0002_mutex_two_threads_no_lock_overlap.c',
    'source/suites/sync/mutex/test_0003_mutex_two_threads_with_lock_overlap.c',
    'source/suites/sync/mutex/test_0004_mutex_multiple_threads_same_priority.c',
    'source/suites/sync/mutex/test_0005_mutex_multiple_threads_different_priority.c',
    'source/suites/sync/remutex/test_0001_remutex_lock_unlock_single_thread.c',
    'source/suites/sync/remutex/test_0002_remutex_two_threads_no_lock_overlap.c',
    'source/suites/sync/remutex/test_0003_remutex_two_threads_with_lock_overlap.c',
    'source/suites/sync/remutex/test_0004_remutex_multiple_threads_same_priority.c',
    'source/suites/sync/remutex/test_0005_remutex_multiple_threads_different_priority.c',
    'source/suites/sync/remutex/test_0006_remutex_reentrancy_single_thread.c',
    'source/suites/sync/remutex/test_0007_remutex_stays_inside_its_own_storage.c',
    'source/suites/sync/condvar/suite.h',
    'source/suites/sync/condvar/test_0001_condvar_basic_wait_wake_one.c',
    'source/suites/sync/condvar/test_0002_condvar_wait_timeout_expiry.c',
    'source/suites/sync/condvar/test_0003_condvar_wait_wake_all.c',
    'source/suites/sync/condvar/test_0004_condvar_sequential_wait_signal.c',
    'source/suites/sync/barrier/suite.h',
    'source/suites/sync/barrier/test_0001_barrier_sync_multiple_threads.c',
    'source/suites/sync/rwlock/suite.h',
    'source/suites/sync/rwlock/test_0001_rwlock_read_lock_single_thread.c',
    'source/suites/sync/rwlock/test_0002_rwlock_write_lock_single_thread.c',
    'source/suites/sync/rwlock/test_0003_rwlock_multiple_readers_concurrent.c',
    'source/suites/sync/rwlock/test_0004_rwlock_write_lock_exclusive.c',
    'source/suites/sync/rwlock/test_0005_rwlock_reader_writer_priority.c',
    'source/suites/sync/rwlock/test_0006_rwlock_try_operations.c',
    'source/suites/sync/rwlock/test_0007_rwlock_write_first_unlock.c',
    'source/suites/sync/rwlock/test_0008_rwlock_reads_first_unlock.c',
    'source/suites/sync/rwlock/test_0009_rwlock_mixed_unlock_order.c',
    'source/suites/sync/rwlock/test_0010_rwlock_ownership_checks.c',
    'source/suites/sync/semaphore/suite.h',
    'source/suites/sync/semaphore/test_0001_semaphore_wait_signal_single_thread.c',
    'source/suites/sync/semaphore/test_0002_semaphore_multiple_threads_initial_count.c',
    'source/suites/sync/semaphore/test_0003_semaphore_producer_consumer.c',
    'source/suites/sync/oneshot/suite.h',
    'source/suites/sync/oneshot/test_0001_oneshot_two_threads_send_recv.c',
    'source/suites/sync/oneshot/test_0002_oneshot_recv_sender_dropped.c',
    'source/suites/sync/oneshot/test_0003_oneshot_send_receiver_dropped.c',
    'source/suites/sync/main.c',
)
elf = executable(name + '.elf', src,
                 dependencies : [nx_dep, nx_std_sync_dep],
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_base + ['-Wl,-Map,@0@/@1@.map'.format(meson.current_build_dir(), name)],
                 pie : true)
custom_target(name + '.lst',
              input : elf, output : name + '.lst',
              command : [nm, '-CSn', '@INPUT@'], capture : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── nx-tests-fs ───────────────────────────────────────────────────────────
# Runs unattended, and touches the SD card: a card that is missing, full or
# write-protected fails this whole suite for reasons that have nothing to do
# with the code, which is why it is worth knowing that a run that fails here
# failed here alone. It links either way — with
# `use_nx_service_fs` and `use_nx_fsdev` off the `fs*` commands and the devoptab
# layer above them resolve to libnx, which is the baseline to compare our
# implementation against.
name = 'nx-tests-fs'
src = files(
    'source/rig.h',
    'source/suites/handback.h',
    'source/suites/tap.h',
    'source/suites/tap.c',
    'source/suites/fs/sdmc/fixture.h',
    'source/suites/fs/sdmc/suite.h',
    'source/suites/fs/sdmc/test_0001_write_then_read_returns_same_bytes.c',
    'source/suites/fs/sdmc/test_0002_stat_reports_size_and_type.c',
    'source/suites/fs/sdmc/test_0003_seek_moves_the_read_position.c',
    'source/suites/fs/sdmc/test_0004_opendir_lists_every_entry_once.c',
    'source/suites/fs/sdmc/test_0005_rename_moves_the_entry.c',
    'source/suites/fs/sdmc/test_0006_unlink_and_rmdir_remove_entries.c',
    'source/suites/fs/sdmc/test_0007_opening_a_missing_file_reports_enoent.c',
    'source/suites/fs/sdmc/test_0008_chdir_makes_relative_paths_resolve.c',
    'source/suites/fs/sdmc/test_0009_getcwd_reports_the_working_directory.c',
    'source/suites/fs/sdmc/test_0010_mkdir_on_an_existing_entry_reports_eexist.c',
    'source/suites/fs/sdmc/test_0011_rmdir_on_a_non_empty_directory_fails.c',
    'source/suites/fs/sdmc/test_0012_append_mode_writes_at_the_end.c',
    'source/suites/fs/sdmc/test_0013_ftruncate_shortens_the_file.c',
    'source/suites/fs/sdmc/test_0014_write_larger_than_one_command_round_trips.c',
    'source/suites/fs/sdmc/test_0015_two_descriptors_on_one_file_are_independent.c',
    'source/suites/fs/sdmc/test_0016_statvfs_reports_the_card_capacity.c',
    'source/suites/fs/sdmc/test_0017_exclusive_create_on_an_existing_file_reports_eexist.c',
    'source/suites/fs/sdmc/test_0018_dup_gives_a_second_descriptor_onto_one_file.c',
    'source/suites/fs/sdmc/test_0019_closing_one_duplicate_leaves_the_file_open.c',
    'source/suites/fs/sdmc/test_0020_dup2_rebinds_a_descriptor_onto_another_file.c',
    'source/suites/fs/savedata/fixture.h',
    'source/suites/fs/savedata/suite.h',
    'source/suites/fs/savedata/test_0001_open_save_data_info_reader_over_the_user_space_succeeds.c',
    'source/suites/fs/savedata/test_0002_save_data_info_reader_read_walks_to_exhaustion.c',
    'source/suites/fs/savedata/test_0003_open_save_data_info_reader_over_every_space_succeeds.c',
    'source/suites/fs/savedata/test_0004_open_save_data_info_reader_with_filter_reports_only_account_saves.c',
    'source/suites/fs/savedata/test_0005_open_save_data_opens_an_account_save.c',
    'source/suites/fs/savedata/test_0006_open_save_data_read_only_opens_the_same_save.c',
    'source/suites/fs/savedata/test_0007_open_save_data_for_an_unknown_application_fails.c',
    'source/suites/fs/savedata/test_0008_fsdev_mount_save_data_lists_the_save_root.c',
    'source/suites/fs/main.c',
)
elf = executable(name + '.elf', src,
                 dependencies : [nx_dep, nx_std_sync_dep],
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_base + ['-Wl,-Map,@0@/@1@.map'.format(meson.current_build_dir(), name)],
                 pie : true)
custom_target(name + '.lst',
              input : elf, output : name + '.lst',
              command : [nm, '-CSn', '@INPUT@'], capture : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── nx-tests-net ──────────────────────────────────────────────────────────
# The socket surface, kept in its own binary because it brings up a network
# stack: the driver claims transfer memory and several IPC sessions for as long
# as it is up, and a console where the service refuses to start would fail every
# case for a reason that has nothing to do with the code under test. Every test
# runs over loopback, so a run needs no network and no peer. It links either
# way: with `use_nx_sys_net` off the calls resolve to libnx's own socket driver,
# the baseline to compare against.
name = 'nx-tests-net'
src = files(
    'source/rig.h',
    'source/suites/handback.h',
    'source/suites/tap.h',
    'source/suites/tap.c',
    'source/suites/net/resolve/fixture.h',
    'source/suites/net/resolve/suite.h',
    'source/suites/net/resolve/test_0001_getaddrinfo_with_a_numeric_host_returns_that_address.c',
    'source/suites/net/resolve/test_0002_getaddrinfo_with_no_host_and_no_service_fails.c',
    'source/suites/net/resolve/test_0003_getnameinfo_describes_an_address.c',
    'source/suites/net/resolve/test_0004_gai_strerror_describes_every_code.c',
    'source/suites/net/resolve/test_0005_gethostbyname_with_a_numeric_host_returns_that_address.c',
    'source/suites/net/resolve/test_0006_getaddrinfo_resolves_a_public_name.c',
    'source/suites/net/socket/fixture.h',
    'source/suites/net/socket/suite.h',
    'source/suites/net/socket/test_0001_socket_create_and_close_round_trips.c',
    'source/suites/net/socket/test_0002_bind_then_getsockname_reports_the_address.c',
    'source/suites/net/socket/test_0003_tcp_loopback_round_trips_a_payload.c',
    'source/suites/net/socket/test_0004_read_and_write_reach_the_socket.c',
    'source/suites/net/socket/test_0005_udp_loopback_round_trips_a_datagram.c',
    'source/suites/net/socket/test_0006_nonblocking_accept_on_an_idle_listener_reports_eagain.c',
    'source/suites/net/socket/test_0007_poll_reports_readability_when_data_arrives.c',
    'source/suites/net/socket/test_0008_select_reports_readability_when_data_arrives.c',
    'source/suites/net/socket/test_0009_socket_calls_on_a_non_socket_report_enotsock.c',
    'source/suites/net/socket/test_0010_sockopt_round_trips_a_value.c',
    'source/suites/net/socket/test_0011_getpeername_reports_the_connected_peer.c',
    'source/suites/net/socket/test_0012_shutdown_ends_the_peer_receive.c',
    'source/suites/net/main.c',
)
elf = executable(name + '.elf', src,
                 dependencies : [nx_dep, nx_std_sync_dep],
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_base + ['-Wl,-Map,@0@/@1@.map'.format(meson.current_build_dir(), name)],
                 pie : true)
custom_target(name + '.lst',
              input : elf, output : name + '.lst',
              command : [nm, '-CSn', '@INPUT@'], capture : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── nx-tests-applet-album ─────────────────────────────────────────────────
# Interactive: each Album launch blocks until a person leaves the applet, so
# this cannot join the unattended suite. It links either way — with
# `use_nx_service_applet_album` off the calls resolve to libnx, which is the
# baseline to compare our implementation against.
name = 'nx-tests-applet-album'
src = files('source/suites/applet_album/main.c')
elf = executable(name + '.elf', src,
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_base + ['-Wl,-Map,@0@/@1@.map'.format(meson.current_build_dir(), name)],
                 pie : true)
custom_target(name + '.lst',
              input : elf, output : name + '.lst',
              command : [nm, '-CSn', '@INPUT@'], capture : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── nx-tests-applet-err ───────────────────────────────────────────────────
# Interactive: the error applet blocks until a person dismisses the dialog, so
# this cannot join the unattended suite. It links either way — with
# `use_nx_service_applet_err` off the calls resolve to libnx, which is the
# baseline to compare our implementation against.
name = 'nx-tests-applet-err'
src = files('source/suites/applet_err/main.c')
elf = executable(name + '.elf', src,
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_base + ['-Wl,-Map,@0@/@1@.map'.format(meson.current_build_dir(), name)],
                 pie : true)
custom_target(name + '.lst',
              input : elf, output : name + '.lst',
              command : [nm, '-CSn', '@INPUT@'], capture : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)
