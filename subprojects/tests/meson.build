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
# Test NROs
#---------------------------------------------------------------------------------
# One NRO per area. Anything that runs unattended belongs in `nx-tests`; an area
# that needs a person in front of the console gets its own binary so it never
# blocks the automated suite.

# ── nx-tests ──────────────────────────────────────────────────────────────
# The unattended suite: every test runs to completion and reports pass/fail.
name = 'nx-tests'
src = files(
    'source/rand/suite.h',
    'source/rand/test_0001_rand_get_fills_buffers_with_random_data.c',
    'source/rand/test_0002_rand_get64_returns_different_values.c',
    'source/sync/suite.h',
    'source/sync/mutex/suite.h',
    'source/sync/mutex/test_0001_mutex_lock_unlock_single_thread.c',
    'source/sync/mutex/test_0002_mutex_two_threads_no_lock_overlap.c',
    'source/sync/mutex/test_0003_mutex_two_threads_with_lock_overlap.c',
    'source/sync/mutex/test_0004_mutex_multiple_threads_same_priority.c',
    'source/sync/mutex/test_0005_mutex_multiple_threads_different_priority.c',
    'source/sync/remutex/test_0001_remutex_lock_unlock_single_thread.c',
    'source/sync/remutex/test_0002_remutex_two_threads_no_lock_overlap.c',
    'source/sync/remutex/test_0003_remutex_two_threads_with_lock_overlap.c',
    'source/sync/remutex/test_0004_remutex_multiple_threads_same_priority.c',
    'source/sync/remutex/test_0005_remutex_multiple_threads_different_priority.c',
    'source/sync/remutex/test_0006_remutex_reentrancy_single_thread.c',
    'source/sync/condvar/suite.h',
    'source/sync/condvar/test_0001_condvar_basic_wait_wake_one.c',
    'source/sync/condvar/test_0002_condvar_wait_timeout_expiry.c',
    'source/sync/condvar/test_0003_condvar_wait_wake_all.c',
    'source/sync/condvar/test_0004_condvar_sequential_wait_signal.c',
    'source/sync/barrier/suite.h',
    'source/sync/barrier/test_0001_barrier_sync_multiple_threads.c',
    'source/sync/rwlock/suite.h',
    'source/sync/rwlock/test_0001_rwlock_read_lock_single_thread.c',
    'source/sync/rwlock/test_0002_rwlock_write_lock_single_thread.c',
    'source/sync/rwlock/test_0003_rwlock_multiple_readers_concurrent.c',
    'source/sync/rwlock/test_0004_rwlock_write_lock_exclusive.c',
    'source/sync/rwlock/test_0005_rwlock_reader_writer_priority.c',
    'source/sync/rwlock/test_0006_rwlock_try_operations.c',
    'source/sync/rwlock/test_0007_rwlock_write_first_unlock.c',
    'source/sync/rwlock/test_0008_rwlock_reads_first_unlock.c',
    'source/sync/rwlock/test_0009_rwlock_mixed_unlock_order.c',
    'source/sync/rwlock/test_0010_rwlock_ownership_checks.c',
    'source/sync/semaphore/suite.h',
    'source/sync/semaphore/test_0001_semaphore_wait_signal_single_thread.c',
    'source/sync/semaphore/test_0002_semaphore_multiple_threads_initial_count.c',
    'source/sync/semaphore/test_0003_semaphore_producer_consumer.c',
    'source/sync/oneshot/suite.h',
    'source/sync/oneshot/test_0001_oneshot_two_threads_send_recv.c',
    'source/sync/oneshot/test_0002_oneshot_recv_sender_dropped.c',
    'source/sync/oneshot/test_0003_oneshot_send_receiver_dropped.c',
    'source/thread/suite.h',
    'source/thread/test_0001_detach_running_thread_self_reclaims.c',
    'source/thread/test_0002_detach_after_exit_reclaims.c',
    'source/main.c',
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
# Runs unattended like `nx-tests`, but kept in its own binary because it touches
# the SD card: a card that is missing, full or write-protected fails this whole
# suite for reasons that have nothing to do with the code, and that must not be
# able to take the main suite down with it. It links either way — with
# `use_nx_service_fs` off the `fs*` commands resolve to libnx, which is the
# baseline to compare our implementation against.
name = 'nx-tests-fs'
src = files(
    'source/fs/sdmc/fixture.h',
    'source/fs/sdmc/suite.h',
    'source/fs/sdmc/test_0001_write_then_read_returns_same_bytes.c',
    'source/fs/sdmc/test_0002_stat_reports_size_and_type.c',
    'source/fs/sdmc/test_0003_seek_moves_the_read_position.c',
    'source/fs/sdmc/test_0004_opendir_lists_every_entry_once.c',
    'source/fs/sdmc/test_0005_rename_moves_the_entry.c',
    'source/fs/sdmc/test_0006_unlink_and_rmdir_remove_entries.c',
    'source/fs/sdmc/test_0007_opening_a_missing_file_reports_enoent.c',
    'source/fs/main.c',
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

# ── nx-tests-applet-err ───────────────────────────────────────────────────
# Interactive: the error applet blocks until a person dismisses the dialog, so
# this cannot join the unattended suite. It links either way — with
# `use_nx_service_applet_err` off the calls resolve to libnx, which is the
# baseline to compare our implementation against.
name = 'nx-tests-applet-err'
src = files('source/applet_err/main.c')
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
