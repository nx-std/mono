project('nx-tests', 'c',
        version : '1.0.0',
        meson_version : '>= 1.3.0',
        default_options : [
            'buildtype=debugoptimized', # -g -O2
            'warning_level=1', # -Wall
        ])

# Project metadata
name = 'nx-tests'
author = 'LNSD'
version = meson.project_version()

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
nx_std_sync_proj = subproject('nx-std-sync')
nx_std_sync_dep = nx_std_sync_proj.get_variable('nx_std_sync_dep')

#---------------------------------------------------------------------------------
# ELF
#---------------------------------------------------------------------------------
# Source files
c_src = files(
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
    'source/main.c',
)

# Compiler (and linker) options
arch_opts = ['-march=armv8-a+crc+crypto', '-mtune=cortex-a57', '-mtp=soft', '-fPIE']
c_opts = ['-ffunction-sections'] + arch_opts + ['-D__SWITCH__', '-DVERSION="v@0@"'.format(version)]
cpp_opts = c_opts + ['-fno-rtti', '-fno-exceptions']
ld_flags = ['-specs=@0@'.format(nx_switch_specs), '-g'] + arch_opts + ['-Wl,-Map,@0@/@1@.map'.format(meson.current_build_dir(), name)]

# Compile the ELF file
elf = executable(
    '@0@.elf'.format(name),
    c_src,
    dependencies : [nx_dep, nx_std_sync_dep],
    c_args : c_opts,
    cpp_args : cpp_opts,
    link_args : ld_flags,
    pie : true,
)

# Generate the *.lst file
nm = find_program('nm', required : true)
elf_lst = custom_target(
    '@0@.lst'.format(name),
    input : elf,
    output : '@0@.lst'.format(name),
    command : [nm, '-CSn', '@INPUT@'],
    capture : true,
)

#---------------------------------------------------------------------------------
# Post-processing and bundling (NSP/NRO generation)
#---------------------------------------------------------------------------------
## Bundling
fs = import('fs')
bundle_sh = find_program('bundle', required : true)

# Get the NPDM configuration file
npdm_json = get_variable('npdm_json', 'config.json')

# Determine if we're building an NSP or NRO
# The presence of a NPDM file indicates an NSP build
bundle_type = fs.is_file(npdm_json) ? 'NSP' : 'NRO'

bundle_opts = []
if 'NSP' == bundle_type
    # Check for required tools
    find_program('npdmtool', required : true)
    find_program('elf2nso', required : true)
    find_program('build_pfs0', required : true)

    # NSP npdm
    bundle_opts += ['--npdm-json', npdm_json]

    # NSP build
    custom_target('@0@.nsp'.format(name),
                  input : elf,
                  output : '@0@.nsp'.format(name),
                  command : [
                      bundle_sh,
                      '--out-dir', '@OUTDIR@',
                      '--input', '@INPUT@',
                      '--output', '@OUTPUT0@',
                      '--tmp-dir', '@PRIVATE_DIR@'
                  ] + bundle_opts,
                  build_by_default : true)
elif 'NRO' == bundle_type
    # Check for required tools
    find_program('elf2nro', required : true)
    find_program('nacptool', required : true)

    # NRO RomFS
    if is_variable('romfs')
        romfs = get_variable('romfs')
        bundle_opts += ['--romfs', romfs]
    endif

    # NRO icon
    # - If icon is specified, use it
    # - Else, find a 'icon.jpg' file in the current directory
    # - Else, find a '{name}.jpg' file in the current directory
    # - Else, use the default icon (libnx's default icon)
    if is_variable('icon')
        icon = get_variable('icon')
    elif fs.is_file('icon.jpg') # icon.jpg
        icon = 'icon.jpg'
    elif fs.is_file('@0@.jpg'.format(name)) # {name}.jpg
        icon = '@0@.jpg'.format(name)
    elif fs.is_file(nx_default_icon) # libnx's default icon
        icon = nx_default_icon
    else
        error('No icon file found')
    endif
    bundle_opts += ['--icon', icon]

    # NRO nacp generation
    # - If nacp_gen is set, generate nacp
    # - Else, skip nacp generation
    gen_nacp = get_variable('gen_nacp', true)
    if gen_nacp
        bundle_opts += [
            '--name', name,
            '--author', author,
            '--version', version,
        ]
    else
        bundle_opts += ['--no-nacp']
    endif

    # NRO build
    custom_target('@0@.nro'.format(name),
                  input : elf,
                  output : '@0@.nro'.format(name),
                  command : [
                      bundle_sh,
                      '--out-dir', '@OUTDIR@',
                      '--input', '@INPUT@',
                      '--output', '@OUTPUT0@',
                      '--tmp-dir', '@PRIVATE_DIR@'
                  ] + bundle_opts,
                  build_by_default : true)
else
    error('Unsupported bundle type: @0@'.format(bundle_type))
endif
