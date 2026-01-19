project('nx', 'c',
        version : '4.9.0',
        meson_version : '>= 1.4.0',
        default_options : [
            'warning_level=1', # -Wall
            'werror=true', # -Werror
            'cpp_std=gnu++11', # -std=gnu++11
        ])

#---------------------------------------------------------------------------------
# Options
#---------------------------------------------------------------------------------
# The `use_nx` global option is used to enable/disable the function overrides.
# If enabled, and the specialized options are in auto mode, the overrides are enabled.
# If disabled, and the specialized options are in auto mode, the overrides are disabled.
# Individually, the specialized options can override the `use_nx` global option.
use_nx = get_option('use_nx')

use_nx_alloc = get_option('use_nx_alloc').enable_auto_if(use_nx.enabled()).disable_auto_if(use_nx.disabled())
use_nx_rand = get_option('use_nx_rand').enable_auto_if(use_nx.enabled()).disable_auto_if(use_nx.disabled())
use_nx_rt = get_option('use_nx_rt').enable_auto_if(use_nx.enabled()).disable_auto_if(use_nx.disabled())
use_nx_std_sync = get_option('use_nx_std_sync').enable_auto_if(use_nx.enabled()).disable_auto_if(use_nx.disabled())
use_nx_svc = get_option('use_nx_svc').enable_auto_if(use_nx.enabled()).disable_auto_if(use_nx.disabled())
use_nx_sys_mem = get_option('use_nx_sys_mem').enable_auto_if(use_nx.enabled()).disable_auto_if(use_nx.disabled())
use_nx_sys_sync = get_option('use_nx_sys_sync').enable_auto_if(use_nx.enabled()).disable_auto_if(use_nx.disabled())
use_nx_sys_thread_tls = get_option('use_nx_sys_thread_tls').enable_auto_if(use_nx.enabled()).disable_auto_if(use_nx.disabled())
use_nx_sys_thread = get_option('use_nx_sys_thread').enable_auto_if(use_nx.enabled()).disable_auto_if(use_nx.disabled())
use_nx_time = get_option('use_nx_time').enable_auto_if(use_nx.enabled()).disable_auto_if(use_nx.disabled())
use_nx_sf = get_option('use_nx_sf').enable_auto_if(use_nx.enabled()).disable_auto_if(use_nx.disabled())
use_nx_service_apm = get_option('use_nx_service_apm').enable_auto_if(use_nx.enabled()).disable_auto_if(use_nx.disabled())
use_nx_service_set = get_option('use_nx_service_set').enable_auto_if(use_nx.enabled()).disable_auto_if(use_nx.disabled())
use_nx_service_sm = get_option('use_nx_service_sm').enable_auto_if(use_nx.enabled()).disable_auto_if(use_nx.disabled())

#---------------------------------------------------------------------------------
# Dependencies
#---------------------------------------------------------------------------------
deps = []
deps_override_link_args = []

# nx-std
if (use_nx.enabled()
    or use_nx_alloc.enabled() or use_nx_rand.enabled() or use_nx_rt.enabled() or use_nx_std_sync.enabled()
    or use_nx_svc.enabled() or use_nx_sys_mem.enabled() or use_nx_sys_sync.enabled() or use_nx_sys_thread_tls.enabled()
    or use_nx_sys_thread.enabled() or use_nx_time.enabled() or use_nx_sf.enabled()
    or use_nx_service_apm.enabled() or use_nx_service_set.enabled() or use_nx_service_sm.enabled())

    nx_std_proj = subproject(
        'nx-std',
        default_options : {
            'use_nx_alloc' : '@0@'.format(use_nx_alloc),
            'use_nx_rand' : '@0@'.format(use_nx_rand),
            'use_nx_rt' : '@0@'.format(use_nx_rt),
            'use_nx_std_sync' : '@0@'.format(use_nx_std_sync),
            'use_nx_svc' : '@0@'.format(use_nx_svc),
            'use_nx_sys_mem' : '@0@'.format(use_nx_sys_mem),
            'use_nx_sys_sync' : '@0@'.format(use_nx_sys_sync),
            'use_nx_sys_thread_tls' : '@0@'.format(use_nx_sys_thread_tls),
            'use_nx_sys_thread' : '@0@'.format(use_nx_sys_thread),
            'use_nx_time' : '@0@'.format(use_nx_time),
            'use_nx_sf' : '@0@'.format(use_nx_sf),
            'use_nx_service_apm' : '@0@'.format(use_nx_service_apm),
            'use_nx_service_set' : '@0@'.format(use_nx_service_set),
            'use_nx_service_sm' : '@0@'.format(use_nx_service_sm),
        },
    )


    debug('Using nx-std')
    deps += nx_std_proj.get_variable('nx_std_dep')
    deps_override_link_args += nx_std_proj.get_variable('nx_std_dep_override_link_args')
endif

#---------------------------------------------------------------------------------
# Compilation specific files
#---------------------------------------------------------------------------------
# Linker script
nx_switch_ld = meson.current_source_dir() / 'src/nx/switch.ld'

# Spec file
nx_switch_specs = meson.current_source_dir() / 'src/nx/switch.specs'

# Default NRO icon
nx_default_icon = meson.current_source_dir() / 'src/nx/default_icon.jpg'


#---------------------------------------------------------------------------------
# Source files
#---------------------------------------------------------------------------------
# C source files
c_src = files(
    'src/nx/source/applets/album_la.c',
    'src/nx/source/applets/error.c',
    'src/nx/source/applets/friends_la.c',
    'src/nx/source/applets/hid_la.c',
    'src/nx/source/applets/libapplet.c',
    'src/nx/source/applets/mii_la.c',
    'src/nx/source/applets/nfp_la.c',
    'src/nx/source/applets/nifm_la.c',
    'src/nx/source/applets/pctlauth.c',
    'src/nx/source/applets/psel.c',
    'src/nx/source/applets/swkbd.c',
    'src/nx/source/applets/web.c',
    'src/nx/source/audio/driver.c',
    'src/nx/source/audio/mempool.c',
    'src/nx/source/audio/mix_object.c',
    'src/nx/source/audio/sink.c',
    'src/nx/source/audio/voice.c',
    'src/nx/source/crypto/aes.c',
    'src/nx/source/crypto/aes_cbc.c',
    'src/nx/source/crypto/aes_ctr.c',
    'src/nx/source/crypto/aes_xts.c',
    'src/nx/source/crypto/cmac.c',
    'src/nx/source/crypto/hmac.c',
    'src/nx/source/crypto/sha1.c',
    'src/nx/source/crypto/sha256.c',
    'src/nx/source/display/binder.c',
    'src/nx/source/display/buffer_producer.c',
    'src/nx/source/display/default_window.c',
    'src/nx/source/display/framebuffer.c',
    'src/nx/source/display/native_window.c',
    'src/nx/source/display/parcel.c',
    'src/nx/source/kernel/barrier.c',
    'src/nx/source/kernel/condvar.c',
    'src/nx/source/kernel/event.c',
    'src/nx/source/kernel/jit.c',
    'src/nx/source/kernel/levent.c',
    'src/nx/source/kernel/mutex.c',
    'src/nx/source/kernel/random.c',
    'src/nx/source/kernel/rwlock.c',
    'src/nx/source/kernel/semaphore.c',
    'src/nx/source/kernel/shmem.c',
    'src/nx/source/kernel/thread.c',
    'src/nx/source/kernel/tmem.c',
    'src/nx/source/kernel/uevent.c',
    'src/nx/source/kernel/utimer.c',
    'src/nx/source/kernel/virtmem.c',
    'src/nx/source/kernel/wait.c',
    'src/nx/source/nvidia/address_space.c',
    'src/nx/source/nvidia/channel.c',
    'src/nx/source/nvidia/fence.c',
    'src/nx/source/nvidia/gpu.c',
    'src/nx/source/nvidia/gpu_channel.c',
    'src/nx/source/nvidia/ioctl/nvchannel.c',
    'src/nx/source/nvidia/ioctl/nvhost-as-gpu.c',
    'src/nx/source/nvidia/ioctl/nvhost-ctrl.c',
    'src/nx/source/nvidia/ioctl/nvhost-ctrl-gpu.c',
    'src/nx/source/nvidia/ioctl/nvmap.c',
    'src/nx/source/nvidia/map.c',
    'src/nx/source/runtime/alloc.c',
    'src/nx/source/runtime/argv.c',
    'src/nx/source/runtime/btdev.c',
    'src/nx/source/runtime/devices/console.c',
    'src/nx/source/runtime/devices/console_debug.c',
    'src/nx/source/runtime/devices/console_sw.c',
    'src/nx/source/runtime/devices/convert_errno.c',
    'src/nx/source/runtime/devices/fs_dev.c',
    'src/nx/source/runtime/devices/path_buf.c',
    'src/nx/source/runtime/devices/romfs_dev.c',
    'src/nx/source/runtime/devices/socket.c',
    'src/nx/source/runtime/devices/usb_comms.c',
    'src/nx/source/runtime/diag.c',
    'src/nx/source/runtime/dynamic.c',
    'src/nx/source/runtime/env.c',
    'src/nx/source/runtime/hosversion.c',
    'src/nx/source/runtime/init.c',
    'src/nx/source/runtime/nacp.c',
    'src/nx/source/runtime/newlib.c',
    'src/nx/source/runtime/nxlink.c',
    'src/nx/source/runtime/nxlink_stdio.c',
    'src/nx/source/runtime/pad.c',
    'src/nx/source/runtime/resolver.c',
    'src/nx/source/runtime/ringcon.c',
    'src/nx/source/runtime/util/inet_addr.c',
    'src/nx/source/runtime/util/utf/decode_utf16.c',
    'src/nx/source/runtime/util/utf/decode_utf8.c',
    'src/nx/source/runtime/util/utf/encode_utf16.c',
    'src/nx/source/runtime/util/utf/encode_utf8.c',
    'src/nx/source/runtime/util/utf/utf16_to_utf32.c',
    'src/nx/source/runtime/util/utf/utf16_to_utf8.c',
    'src/nx/source/runtime/util/utf/utf32_to_utf16.c',
    'src/nx/source/runtime/util/utf/utf32_to_utf8.c',
    'src/nx/source/runtime/util/utf/utf8_to_utf16.c',
    'src/nx/source/runtime/util/utf/utf8_to_utf32.c',
    'src/nx/source/services/acc.c',
    'src/nx/source/services/apm.c',
    'src/nx/source/services/applet.c',
    'src/nx/source/services/async.c',
    'src/nx/source/services/audctl.c',
    'src/nx/source/services/auddev.c',
    'src/nx/source/services/audin.c',
    'src/nx/source/services/audout.c',
    'src/nx/source/services/audrec.c',
    'src/nx/source/services/audren.c',
    'src/nx/source/services/avm.c',
    'src/nx/source/services/bpc.c',
    'src/nx/source/services/bsd.c',
    'src/nx/source/services/bt.c',
    'src/nx/source/services/btdrv.c',
    'src/nx/source/services/btm.c',
    'src/nx/source/services/btmsys.c',
    'src/nx/source/services/btmu.c',
    'src/nx/source/services/capmtp.c',
    'src/nx/source/services/capsa.c',
    'src/nx/source/services/caps.c',
    'src/nx/source/services/capsc.c',
    'src/nx/source/services/capsdc.c',
    'src/nx/source/services/capssc.c',
    'src/nx/source/services/capssu.c',
    'src/nx/source/services/capsu.c',
    'src/nx/source/services/clkrst.c',
    'src/nx/source/services/csrng.c',
    'src/nx/source/services/ectx.c',
    'src/nx/source/services/fan.c',
    'src/nx/source/services/fatal.c',
    'src/nx/source/services/friends.c',
    'src/nx/source/services/fs.c',
    'src/nx/source/services/fsldr.c',
    'src/nx/source/services/fspr.c',
    'src/nx/source/services/gpio.c',
    'src/nx/source/services/grc.c',
    'src/nx/source/services/hidbus.c',
    'src/nx/source/services/hid.c',
    'src/nx/source/services/hiddbg.c',
    'src/nx/source/services/hidsys.c',
    'src/nx/source/services/htcs.c',
    'src/nx/source/services/hwopus.c',
    'src/nx/source/services/i2c.c',
    'src/nx/source/services/ins.c',
    'src/nx/source/services/irs.c',
    'src/nx/source/services/lbl.c',
    'src/nx/source/services/ldn.c',
    'src/nx/source/services/ldr.c',
    'src/nx/source/services/lp2p.c',
    'src/nx/source/services/lr.c',
    'src/nx/source/services/mii.c',
    'src/nx/source/services/miiimg.c',
    'src/nx/source/services/mm.c',
    'src/nx/source/services/ncm.c',
    'src/nx/source/services/news.c',
    'src/nx/source/services/nfc.c',
    'src/nx/source/services/nifm.c',
    'src/nx/source/services/nim.c',
    'src/nx/source/services/notif.c',
    'src/nx/source/services/ns.c',
    'src/nx/source/services/nv.c',
    'src/nx/source/services/pctl.c',
    'src/nx/source/services/pcv.c',
    'src/nx/source/services/pdm.c',
    'src/nx/source/services/pgl.c',
    'src/nx/source/services/pl.c',
    'src/nx/source/services/pm.c',
    'src/nx/source/services/psc.c',
    'src/nx/source/services/psm.c',
    'src/nx/source/services/ro.c',
    'src/nx/source/services/set.c',
    'src/nx/source/services/sfdnsres.c',
    'src/nx/source/services/sm.c',
    'src/nx/source/services/smm.c',
    'src/nx/source/services/spl.c',
    'src/nx/source/services/spsm.c',
    'src/nx/source/services/ssl.c',
    'src/nx/source/services/tc.c',
    'src/nx/source/services/time.c',
    'src/nx/source/services/ts.c',
    'src/nx/source/services/uart.c',
    'src/nx/source/services/usbds.c',
    'src/nx/source/services/usbhs.c',
    'src/nx/source/services/vi.c',
    'src/nx/source/services/wlaninf.c',
    'src/nx/source/sf/sessionmgr.c',
)

# Assembly source files
s_src = files(
    'src/nx/source/arm/cache.s',
    'src/nx/source/kernel/svc.s',
    'src/nx/source/runtime/exception.s',
    'src/nx/source/runtime/readtp.s',
    'src/nx/source/runtime/switch_crt0.s',
)

# Include dirs
internal_inc = include_directories(
    'src/nx/include',
    'src/nx/include/switch',
    'src/nx/external/bsd/include',
)

#---------------------------------------------------------------------------------
# Data files
#---------------------------------------------------------------------------------
data_src = files(
    'src/nx/data/default_font.bin',
)

# bin2s generator
bin2s = find_program('bin2s', required : true)
bin2s_gen = generator(bin2s,
                      output : ['@BASENAME@_bin.s', '@BASENAME@_bin.h'],
                      arguments : ['-a', '8', '-H', '@BUILD_DIR@/@BASENAME@_bin.h', '@INPUT@'],
                      capture : true,
)

# Process data files
data_gen_src = bin2s_gen.process(data_src)

#---------------------------------------------------------------------------------
# Static library
#---------------------------------------------------------------------------------
# Compiler options
arch_opts = ['-march=armv8-a+crc+crypto', '-mtune=cortex-a57', '-mtp=soft', '-fPIC', '-ftls-model=local-exec']
c_opts = ['-g', '-ffunction-sections', '-fdata-sections'] + arch_opts
c_opts += ['-D__SWITCH__', '-DLIBNX_NO_DEPRECATION']
cpp_opts = ['-fno-rtti', '-fno-exceptions']

nx_lib = static_library('nx',
                        [c_src, s_src, data_gen_src],
                        include_directories : internal_inc,
                        dependencies : deps,
                        c_args : c_opts,
                        cpp_args : cpp_opts)

#---------------------------------------------------------------------------------
# Dependency declaration
#---------------------------------------------------------------------------------
nx_dep = declare_dependency(
    include_directories : include_directories('src/nx/include', 'src/nx/external/bsd/include'),
    link_with : nx_lib,
    link_args : deps_override_link_args,
    dependencies : deps,
)
