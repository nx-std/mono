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

# nx-tests-framework
# The rig's own framework, in a directory of its own beside the sources that use
# it: it serves the suites and the runner and nothing outside this subproject, so
# it is built as part of this one rather than reached as a project of its own.
subdir('framework')

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
    'runner/ledger.h',
    'runner/ledger.c',
    'runner/main.c',
)
elf = executable(name + '.elf', src,
                 dependencies : [nx_dep, nx_netloader_dep, nx_tests_framework_hdrs_dep],
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
    'suites/rand/suite.h',
    'suites/rand/test_0001_rand_get_fills_buffers_with_random_data.c',
    'suites/rand/test_0002_rand_get64_returns_different_values.c',
    'suites/rand/main.c',
)
elf = executable(name + '.elf', src,
                 dependencies : [nx_dep, nx_std_sync_dep, nx_tests_framework_dep],
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
    'suites/rt/cwd/program.h',
    'suites/rt/cwd/suite.h',
    'suites/rt/cwd/test_0001_startup_cwd_is_the_program_directory.c',
    'suites/rt/cwd/test_0002_a_bare_path_resolves_next_to_the_program.c',
    'suites/rt/main.c',
)
elf = executable(name + '.elf', src,
                 dependencies : [nx_dep, nx_std_sync_dep, nx_tests_framework_dep],
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
    'suites/thread/suite.h',
    'suites/thread/test_0001_detach_running_thread_self_reclaims.c',
    'suites/thread/test_0002_detach_after_exit_reclaims.c',
    'suites/thread/test_0003_thread_pointer_is_per_thread_and_preserved.c',
    'suites/thread/main.c',
)
elf = executable(name + '.elf', src,
                 dependencies : [nx_dep, nx_std_sync_dep, nx_tests_framework_dep],
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
    'suites/sync/suite.h',
    'suites/sync/mutex/suite.h',
    'suites/sync/mutex/test_0001_mutex_lock_unlock_single_thread.c',
    'suites/sync/mutex/test_0002_mutex_two_threads_no_lock_overlap.c',
    'suites/sync/mutex/test_0003_mutex_two_threads_with_lock_overlap.c',
    'suites/sync/mutex/test_0004_mutex_multiple_threads_same_priority.c',
    'suites/sync/mutex/test_0005_mutex_multiple_threads_different_priority.c',
    'suites/sync/remutex/test_0001_remutex_lock_unlock_single_thread.c',
    'suites/sync/remutex/test_0002_remutex_two_threads_no_lock_overlap.c',
    'suites/sync/remutex/test_0003_remutex_two_threads_with_lock_overlap.c',
    'suites/sync/remutex/test_0004_remutex_multiple_threads_same_priority.c',
    'suites/sync/remutex/test_0005_remutex_multiple_threads_different_priority.c',
    'suites/sync/remutex/test_0006_remutex_reentrancy_single_thread.c',
    'suites/sync/remutex/test_0007_remutex_stays_inside_its_own_storage.c',
    'suites/sync/condvar/suite.h',
    'suites/sync/condvar/test_0001_condvar_basic_wait_wake_one.c',
    'suites/sync/condvar/test_0002_condvar_wait_timeout_expiry.c',
    'suites/sync/condvar/test_0003_condvar_wait_wake_all.c',
    'suites/sync/condvar/test_0004_condvar_sequential_wait_signal.c',
    'suites/sync/barrier/suite.h',
    'suites/sync/barrier/test_0001_barrier_sync_multiple_threads.c',
    'suites/sync/rwlock/suite.h',
    'suites/sync/rwlock/test_0001_rwlock_read_lock_single_thread.c',
    'suites/sync/rwlock/test_0002_rwlock_write_lock_single_thread.c',
    'suites/sync/rwlock/test_0003_rwlock_multiple_readers_concurrent.c',
    'suites/sync/rwlock/test_0004_rwlock_write_lock_exclusive.c',
    'suites/sync/rwlock/test_0005_rwlock_reader_writer_priority.c',
    'suites/sync/rwlock/test_0006_rwlock_try_operations.c',
    'suites/sync/rwlock/test_0007_rwlock_write_first_unlock.c',
    'suites/sync/rwlock/test_0008_rwlock_reads_first_unlock.c',
    'suites/sync/rwlock/test_0009_rwlock_mixed_unlock_order.c',
    'suites/sync/rwlock/test_0010_rwlock_ownership_checks.c',
    'suites/sync/semaphore/suite.h',
    'suites/sync/semaphore/test_0001_semaphore_wait_signal_single_thread.c',
    'suites/sync/semaphore/test_0002_semaphore_multiple_threads_initial_count.c',
    'suites/sync/semaphore/test_0003_semaphore_producer_consumer.c',
    'suites/sync/oneshot/suite.h',
    'suites/sync/oneshot/test_0001_oneshot_two_threads_send_recv.c',
    'suites/sync/oneshot/test_0002_oneshot_recv_sender_dropped.c',
    'suites/sync/oneshot/test_0003_oneshot_send_receiver_dropped.c',
    'suites/sync/main.c',
)
elf = executable(name + '.elf', src,
                 dependencies : [nx_dep, nx_std_sync_dep, nx_tests_framework_dep],
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
    'suites/fs/sdmc/fixture.h',
    'suites/fs/sdmc/suite.h',
    'suites/fs/sdmc/test_0001_write_then_read_returns_same_bytes.c',
    'suites/fs/sdmc/test_0002_stat_reports_size_and_type.c',
    'suites/fs/sdmc/test_0003_seek_moves_the_read_position.c',
    'suites/fs/sdmc/test_0004_opendir_lists_every_entry_once.c',
    'suites/fs/sdmc/test_0005_rename_moves_the_entry.c',
    'suites/fs/sdmc/test_0006_unlink_and_rmdir_remove_entries.c',
    'suites/fs/sdmc/test_0007_opening_a_missing_file_reports_enoent.c',
    'suites/fs/sdmc/test_0008_chdir_makes_relative_paths_resolve.c',
    'suites/fs/sdmc/test_0009_getcwd_reports_the_working_directory.c',
    'suites/fs/sdmc/test_0010_mkdir_on_an_existing_entry_reports_eexist.c',
    'suites/fs/sdmc/test_0011_rmdir_on_a_non_empty_directory_fails.c',
    'suites/fs/sdmc/test_0012_append_mode_writes_at_the_end.c',
    'suites/fs/sdmc/test_0013_ftruncate_shortens_the_file.c',
    'suites/fs/sdmc/test_0014_write_larger_than_one_command_round_trips.c',
    'suites/fs/sdmc/test_0015_two_descriptors_on_one_file_are_independent.c',
    'suites/fs/sdmc/test_0016_statvfs_reports_the_card_capacity.c',
    'suites/fs/sdmc/test_0017_exclusive_create_on_an_existing_file_reports_eexist.c',
    'suites/fs/sdmc/test_0018_dup_gives_a_second_descriptor_onto_one_file.c',
    'suites/fs/sdmc/test_0019_closing_one_duplicate_leaves_the_file_open.c',
    'suites/fs/sdmc/test_0020_dup2_rebinds_a_descriptor_onto_another_file.c',
    'suites/fs/savedata/fixture.h',
    'suites/fs/savedata/suite.h',
    'suites/fs/savedata/test_0001_open_save_data_info_reader_over_the_user_space_succeeds.c',
    'suites/fs/savedata/test_0002_save_data_info_reader_read_walks_to_exhaustion.c',
    'suites/fs/savedata/test_0003_open_save_data_info_reader_over_every_space_succeeds.c',
    'suites/fs/savedata/test_0004_open_save_data_info_reader_with_filter_reports_only_account_saves.c',
    'suites/fs/savedata/test_0005_open_save_data_opens_an_account_save.c',
    'suites/fs/savedata/test_0006_open_save_data_read_only_opens_the_same_save.c',
    'suites/fs/savedata/test_0007_open_save_data_for_an_unknown_application_fails.c',
    'suites/fs/savedata/test_0008_fsdev_mount_save_data_lists_the_save_root.c',
    'suites/fs/main.c',
)
elf = executable(name + '.elf', src,
                 dependencies : [nx_dep, nx_std_sync_dep, nx_tests_framework_dep],
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

# ── nx-tests-romfs ────────────────────────────────────────────────────────
# Runs unattended, and carries its own fixture: the tree under `suites/romfs/assets`
# is packed into this NRO, so the suite needs no SD card contents and no network.
# It does need to be launched from a path it can find itself by — an NRO reaches
# its own image by opening the file named on its command line — so a run says as
# much about the runtime and the descriptor table as about the image reader. It
# links either way: with `use_nx_romfs` off the `romfs*` surface resolves to
# libnx, which is the baseline to compare our implementation against — with one
# case that fails there by construction rather than by regression, since libnx's
# romfs lets a name be mounted twice and this crate refuses.
name = 'nx-tests-romfs'
src = files(
    'suites/romfs/fixture.h',
    'suites/romfs/suite.h',
    'suites/romfs/test_0001_romfs_init_mounts_the_programs_own_image.c',
    'suites/romfs/test_0002_reading_a_file_returns_the_bundled_bytes.c',
    'suites/romfs/test_0003_stat_reports_size_and_type.c',
    'suites/romfs/test_0004_opendir_lists_every_entry_once.c',
    'suites/romfs/test_0005_seek_moves_the_read_position.c',
    'suites/romfs/test_0006_a_file_larger_than_one_read_round_trips.c',
    'suites/romfs/test_0007_opening_a_missing_file_reports_enoent.c',
    'suites/romfs/test_0008_opening_for_writing_is_refused.c',
    'suites/romfs/test_0009_chdir_makes_relative_paths_resolve.c',
    'suites/romfs/test_0010_a_non_ascii_name_resolves.c',
    'suites/romfs/test_0011_unmounting_makes_the_paths_stop_resolving.c',
    'suites/romfs/test_0012_mounting_a_name_twice_is_refused.c',
    'suites/romfs/main.c',
)
elf = executable(name + '.elf', src,
                 dependencies : [nx_dep, nx_std_sync_dep, nx_tests_framework_dep],
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_base + ['-Wl,-Map,@0@/@1@.map'.format(meson.current_build_dir(), name)],
                 pie : true)
custom_target(name + '.lst',
              input : elf, output : name + '.lst',
              command : [nm, '-CSn', '@INPUT@'], capture : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--romfs', meson.current_source_dir() / 'suites/romfs/assets'],
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
    'suites/net/resolve/fixture.h',
    'suites/net/resolve/suite.h',
    'suites/net/resolve/test_0001_getaddrinfo_with_a_numeric_host_returns_that_address.c',
    'suites/net/resolve/test_0002_getaddrinfo_with_no_host_and_no_service_fails.c',
    'suites/net/resolve/test_0003_getnameinfo_describes_an_address.c',
    'suites/net/resolve/test_0004_gai_strerror_describes_every_code.c',
    'suites/net/resolve/test_0005_gethostbyname_with_a_numeric_host_returns_that_address.c',
    'suites/net/resolve/test_0006_getaddrinfo_resolves_a_public_name.c',
    'suites/net/socket/fixture.h',
    'suites/net/socket/suite.h',
    'suites/net/socket/test_0001_socket_create_and_close_round_trips.c',
    'suites/net/socket/test_0002_bind_then_getsockname_reports_the_address.c',
    'suites/net/socket/test_0003_tcp_loopback_round_trips_a_payload.c',
    'suites/net/socket/test_0004_read_and_write_reach_the_socket.c',
    'suites/net/socket/test_0005_udp_loopback_round_trips_a_datagram.c',
    'suites/net/socket/test_0006_nonblocking_accept_on_an_idle_listener_reports_eagain.c',
    'suites/net/socket/test_0007_poll_reports_readability_when_data_arrives.c',
    'suites/net/socket/test_0008_select_reports_readability_when_data_arrives.c',
    'suites/net/socket/test_0009_socket_calls_on_a_non_socket_report_enotsock.c',
    'suites/net/socket/test_0010_sockopt_round_trips_a_value.c',
    'suites/net/socket/test_0011_getpeername_reports_the_connected_peer.c',
    'suites/net/socket/test_0012_shutdown_ends_the_peer_receive.c',
    'suites/net/main.c',
)
elf = executable(name + '.elf', src,
                 dependencies : [nx_dep, nx_std_sync_dep, nx_tests_framework_dep],
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
src = files('suites/applet_album/main.c')
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
src = files('suites/applet_err/main.c')
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
