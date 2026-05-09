project('switch-examples', 'cpp', 'c',
        default_options : ['cpp_std=gnu++17'],
        meson_version : '>=1.4.0')

#---------------------------------------------------------------------------------
# Dependencies
#---------------------------------------------------------------------------------
# libnx
if get_option('use_libnx_dkp').enabled()
  nx_proj = subproject('libnx-dkp')
else
  nx_proj = subproject('libnx')
endif
nx_dep          = nx_proj.get_variable('nx_dep')
nx_switch_specs = nx_proj.get_variable('nx_switch_specs')

# libdeko3d (release + debug variants)
deko3d_proj = subproject('libdeko3d-dkp')
deko3d_dep  = deko3d_proj.get_variable('deko3d_dep')
deko3dd_dep = deko3d_proj.get_variable('deko3dd_dep')

# pkg-config dependencies (devkitPro portlibs)
egl_dep        = dependency('egl',        method : 'pkg-config', static : true)
glapi_dep      = dependency('glapi',      method : 'pkg-config', static : true)
glesv2_dep     = dependency('glesv2',     method : 'pkg-config', static : true)
freetype2_dep  = dependency('freetype2',  method : 'pkg-config', static : true)
opusfile_dep   = dependency('opusfile',   method : 'pkg-config', static : true)
curl_dep       = dependency('libcurl',    method : 'pkg-config', static : true)
sdl2_dep       = dependency('SDL2',       method : 'pkg-config', static : true)
sdl2_mixer_dep = dependency('SDL2_mixer', method : 'pkg-config', static : true)
sdl2_image_dep = dependency('SDL2_image', method : 'pkg-config', static : true)
sdl2_ttf_dep   = dependency('SDL2_ttf',   method : 'pkg-config', static : true)

# libglad and libdrm_nouveau pkg-config files require $(DEVKITPRO) prefix substitution
dkp_prefix = ['prefix', get_option('devkitpro') / 'portlibs/switch']

glad_pc     = dependency('libglad', method : 'pkg-config', static : true)
glad_incdir = glad_pc.get_variable(pkgconfig : 'includedir', pkgconfig_define : dkp_prefix)
glad_libdir = glad_pc.get_variable(pkgconfig : 'libdir',     pkgconfig_define : dkp_prefix)
glad_dep    = declare_dependency(
  include_directories : include_directories(glad_incdir),
  link_args : ['-L@0@'.format(glad_libdir), '-lglad'],
)

drm_nouveau_pc     = dependency('libdrm_nouveau', method : 'pkg-config', static : true)
drm_nouveau_incdir = drm_nouveau_pc.get_variable(pkgconfig : 'includedir', pkgconfig_define : dkp_prefix)
drm_nouveau_libdir = drm_nouveau_pc.get_variable(pkgconfig : 'libdir',     pkgconfig_define : dkp_prefix)
drm_nouveau_dep    = declare_dependency(
  include_directories : include_directories(drm_nouveau_incdir),
  link_args : ['-L@0@'.format(drm_nouveau_libdir), '-ldrm_nouveau'],
)

#---------------------------------------------------------------------------------
# Compiler options
#---------------------------------------------------------------------------------
arch_opts = ['-march=armv8-a+crc+crypto', '-mtune=cortex-a57', '-mtp=soft', '-fPIE']
c_opts    = ['-g', '-Wall', '-O2', '-ffunction-sections'] + arch_opts + ['-D__SWITCH__']
cpp_opts  = c_opts + ['-fno-rtti', '-fno-exceptions']
ld_args   = ['-specs=@0@'.format(nx_switch_specs), '-g', '-lm'] + arch_opts

#---------------------------------------------------------------------------------
# Post-processing and bundling (NSP/NRO generation)
#---------------------------------------------------------------------------------
# Check for required tools
find_program('elf2nro',  required : true)
find_program('nacptool', required : true)
uam_prog  = find_program('uam',    required : true)
bundle_sh = find_program('bundle', required : true)

## Bundling
src         = meson.current_source_dir() / 'src'
default_ico = '/opt/devkitpro/libnx/default_icon.jpg'
bundle_base = [bundle_sh,
               '--out-dir', '@OUTDIR@', '--input', '@INPUT@',
               '--output',  '@OUTPUT0@', '--tmp-dir', '@PRIVATE_DIR@',
               '--icon',    files(default_ico),
               '--version', '1.0.0']

#---------------------------------------------------------------------------------
# Examples
#---------------------------------------------------------------------------------
# ── account ───────────────────────────────────────────────────────────────
name = 'account'
elf = executable(name + '.elf', files(src / 'account/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── alarm-notifications ───────────────────────────────────────────────────
name = 'alarm-notifications'
elf = executable(name + '.elf', files(src / 'alarm-notifications/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── app_controldata ───────────────────────────────────────────────────────
name = 'app_controldata'
elf = executable(name + '.elf', files(src / 'app_controldata/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── applet/app-playstats ──────────────────────────────────────────────────
name = 'app-playstats'
elf = executable(name + '.elf', files(src / 'applet/app-playstats/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── applet/libapplets/error ───────────────────────────────────────────────
name = 'libapplets-error'
elf = executable(name + '.elf', files(src / 'applet/libapplets/error/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── applet/libapplets/pctlauth ────────────────────────────────────────────
name = 'libapplets-pctlauth'
elf = executable(name + '.elf', files(src / 'applet/libapplets/pctlauth/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── applet/libapplets/software-keyboard ───────────────────────────────────
name = 'libapplets-software_keyboard'
elf = executable(name + '.elf', files(src / 'applet/libapplets/software-keyboard/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── applet/libapplets/web ─────────────────────────────────────────────────
name = 'libapplets-web'
elf = executable(name + '.elf', files(src / 'applet/libapplets/web/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── applet/light-sensor ───────────────────────────────────────────────────
name = 'light-sensor'
elf = executable(name + '.elf', files(src / 'applet/light-sensor/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── applet/lockexit ───────────────────────────────────────────────────────
name = 'lockexit'
elf = executable(name + '.elf', files(src / 'applet/lockexit/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── applet/recording ──────────────────────────────────────────────────────
name = 'recording'
elf = executable(name + '.elf', files(src / 'applet/recording/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── applet/screenshot-overlay ─────────────────────────────────────────────
name = 'screenshot-overlay'
elf = executable(name + '.elf', files(src / 'applet/screenshot-overlay/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── applet/vrmode ─────────────────────────────────────────────────────────
name = 'vrmode'
elf = executable(name + '.elf', files(src / 'applet/vrmode/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── audio/audren-simple ───────────────────────────────────────────────────
name = 'audio-audren_simple'
elf = executable(name + '.elf', files(src / 'audio/audren-simple/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── audio/echo ────────────────────────────────────────────────────────────
name = 'audio-echo'
elf = executable(name + '.elf', files(src / 'audio/echo/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── audio/hwopus-decoder ──────────────────────────────────────────────────
name = 'audio-hwopus_decoder'
elf = executable(name + '.elf', files(src / 'audio/hwopus-decoder/source/main.c'),
                 dependencies : [nx_dep, opusfile_dep],
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── audio/playtone ────────────────────────────────────────────────────────
name = 'audio-playtone'
elf = executable(name + '.elf', files(src / 'audio/playtone/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── audio/sdl2-audio ──────────────────────────────────────────────────────
name = 'audio-sdl2_audio'
elf = executable(name + '.elf', files(src / 'audio/sdl2-audio/source/main.c'),
                 dependencies : [nx_dep, sdl2_mixer_dep],
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew', '--romfs', src / 'audio/sdl2-audio/romfs'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── exception-handler ─────────────────────────────────────────────────────
name = 'exception-handler'
elf = executable(name + '.elf', files(src / 'exception-handler/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── fs/romfs ──────────────────────────────────────────────────────────────
name = 'romfs'
elf = executable(name + '.elf', files(src / 'fs/romfs/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew', '--romfs', src / 'fs/romfs/romfs'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── fs/save ───────────────────────────────────────────────────────────────
name = 'save'
elf = executable(name + '.elf', files(src / 'fs/save/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── fs/sdmc ───────────────────────────────────────────────────────────────
name = 'sdmc'
elf = executable(name + '.elf', files(src / 'fs/sdmc/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── graphics/deko3d/deko_basic ────────────────────────────────────────────
name = 'graphics-deko3d-deko_basic'
elf = executable(name + '.elf', files(src / 'graphics/deko3d/deko_basic/source/main.c'),
                 dependencies : [nx_dep, deko3dd_dep],
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
sh_graphics_deko3d_deko_basic_color_fsh = custom_target(name + '-color_fsh.dksh',
  input : files(src / 'graphics/deko3d/deko_basic/source/color_fsh.glsl'),
  output : name + '-color_fsh.dksh',
  command : [uam_prog, '-s', 'frag', '-o', '@OUTPUT@', '@INPUT@'],
  build_by_default : true)
sh_graphics_deko3d_deko_basic_triangle_vsh = custom_target(name + '-triangle_vsh.dksh',
  input : files(src / 'graphics/deko3d/deko_basic/source/triangle_vsh.glsl'),
  output : name + '-triangle_vsh.dksh',
  command : [uam_prog, '-s', 'vert', '-o', '@OUTPUT@', '@INPUT@'],
  build_by_default : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── graphics/deko3d/deko_console ──────────────────────────────────────────
name = 'graphics-deko3d-deko_console'
sources = files(
  src / 'graphics/deko3d/deko_console/source/main.c',
  src / 'graphics/deko3d/deko_console/source/gpu_console.c',
  src / 'graphics/deko3d/deko_console/source/startup.c',
)
elf = executable(name + '.elf', sources,
                 dependencies : [nx_dep, deko3d_dep],
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
sh_graphics_deko3d_deko_console_console_fsh = custom_target(name + '-console_fsh.dksh',
  input : files(src / 'graphics/deko3d/deko_console/source/console_fsh.glsl'),
  output : name + '-console_fsh.dksh',
  command : [uam_prog, '-s', 'frag', '-o', '@OUTPUT@', '@INPUT@'],
  build_by_default : true)
sh_graphics_deko3d_deko_console_console_vsh = custom_target(name + '-console_vsh.dksh',
  input : files(src / 'graphics/deko3d/deko_console/source/console_vsh.glsl'),
  output : name + '-console_vsh.dksh',
  command : [uam_prog, '-s', 'vert', '-o', '@OUTPUT@', '@INPUT@'],
  build_by_default : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── graphics/deko3d/deko_examples ─────────────────────────────────────────
name = 'graphics-deko3d-deko_examples'
sources = files(
  src / 'graphics/deko3d/deko_examples/source/main.cpp',
  src / 'graphics/deko3d/deko_examples/source/Example01_SimpleSetup.cpp',
  src / 'graphics/deko3d/deko_examples/source/Example02_Triangle.cpp',
  src / 'graphics/deko3d/deko_examples/source/Example03_Cube.cpp',
  src / 'graphics/deko3d/deko_examples/source/Example04_TexturedCube.cpp',
  src / 'graphics/deko3d/deko_examples/source/Example05_Tessellation.cpp',
  src / 'graphics/deko3d/deko_examples/source/Example06_Multisampling.cpp',
  src / 'graphics/deko3d/deko_examples/source/Example07_MeshLighting.cpp',
  src / 'graphics/deko3d/deko_examples/source/Example08_DeferredShading.cpp',
  src / 'graphics/deko3d/deko_examples/source/Example09_SimpleCompute.cpp',
  src / 'graphics/deko3d/deko_examples/source/SampleFramework/CApplication.cpp',
  src / 'graphics/deko3d/deko_examples/source/SampleFramework/CExternalImage.cpp',
  src / 'graphics/deko3d/deko_examples/source/SampleFramework/CIntrusiveTree.cpp',
  src / 'graphics/deko3d/deko_examples/source/SampleFramework/CMemPool.cpp',
  src / 'graphics/deko3d/deko_examples/source/SampleFramework/CShader.cpp',
  src / 'graphics/deko3d/deko_examples/source/SampleFramework/FileLoader.cpp',
  src / 'graphics/deko3d/deko_examples/source/SampleFramework/startup.cpp',
)
elf = executable(name + '.elf', sources,
                 dependencies : [nx_dep, deko3dd_dep],
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
sh_graphics_deko3d_deko_examples_basic_deferred_fsh = custom_target(name + '-basic_deferred_fsh.dksh',
  input : files(src / 'graphics/deko3d/deko_examples/source/basic_deferred_fsh.glsl'),
  output : name + '-basic_deferred_fsh.dksh',
  command : [uam_prog, '-s', 'frag', '-o', '@OUTPUT@', '@INPUT@'],
  build_by_default : true)
sh_graphics_deko3d_deko_examples_basic_lighting_fsh = custom_target(name + '-basic_lighting_fsh.dksh',
  input : files(src / 'graphics/deko3d/deko_examples/source/basic_lighting_fsh.glsl'),
  output : name + '-basic_lighting_fsh.dksh',
  command : [uam_prog, '-s', 'frag', '-o', '@OUTPUT@', '@INPUT@'],
  build_by_default : true)
sh_graphics_deko3d_deko_examples_basic_vsh = custom_target(name + '-basic_vsh.dksh',
  input : files(src / 'graphics/deko3d/deko_examples/source/basic_vsh.glsl'),
  output : name + '-basic_vsh.dksh',
  command : [uam_prog, '-s', 'vert', '-o', '@OUTPUT@', '@INPUT@'],
  build_by_default : true)
sh_graphics_deko3d_deko_examples_color_fsh = custom_target(name + '-color_fsh.dksh',
  input : files(src / 'graphics/deko3d/deko_examples/source/color_fsh.glsl'),
  output : name + '-color_fsh.dksh',
  command : [uam_prog, '-s', 'frag', '-o', '@OUTPUT@', '@INPUT@'],
  build_by_default : true)
sh_graphics_deko3d_deko_examples_composition_fsh = custom_target(name + '-composition_fsh.dksh',
  input : files(src / 'graphics/deko3d/deko_examples/source/composition_fsh.glsl'),
  output : name + '-composition_fsh.dksh',
  command : [uam_prog, '-s', 'frag', '-o', '@OUTPUT@', '@INPUT@'],
  build_by_default : true)
sh_graphics_deko3d_deko_examples_composition_vsh = custom_target(name + '-composition_vsh.dksh',
  input : files(src / 'graphics/deko3d/deko_examples/source/composition_vsh.glsl'),
  output : name + '-composition_vsh.dksh',
  command : [uam_prog, '-s', 'vert', '-o', '@OUTPUT@', '@INPUT@'],
  build_by_default : true)
sh_graphics_deko3d_deko_examples_tess_simple_tcsh = custom_target(name + '-tess_simple_tcsh.dksh',
  input : files(src / 'graphics/deko3d/deko_examples/source/tess_simple_tcsh.glsl'),
  output : name + '-tess_simple_tcsh.dksh',
  command : [uam_prog, '-s', 'tess_ctrl', '-o', '@OUTPUT@', '@INPUT@'],
  build_by_default : true)
sh_graphics_deko3d_deko_examples_tess_simple_tesh = custom_target(name + '-tess_simple_tesh.dksh',
  input : files(src / 'graphics/deko3d/deko_examples/source/tess_simple_tesh.glsl'),
  output : name + '-tess_simple_tesh.dksh',
  command : [uam_prog, '-s', 'tess_eval', '-o', '@OUTPUT@', '@INPUT@'],
  build_by_default : true)
sh_graphics_deko3d_deko_examples_texture_fsh = custom_target(name + '-texture_fsh.dksh',
  input : files(src / 'graphics/deko3d/deko_examples/source/texture_fsh.glsl'),
  output : name + '-texture_fsh.dksh',
  command : [uam_prog, '-s', 'frag', '-o', '@OUTPUT@', '@INPUT@'],
  build_by_default : true)
sh_graphics_deko3d_deko_examples_transform_normal_vsh = custom_target(name + '-transform_normal_vsh.dksh',
  input : files(src / 'graphics/deko3d/deko_examples/source/transform_normal_vsh.glsl'),
  output : name + '-transform_normal_vsh.dksh',
  command : [uam_prog, '-s', 'vert', '-o', '@OUTPUT@', '@INPUT@'],
  build_by_default : true)
sh_graphics_deko3d_deko_examples_transform_vsh = custom_target(name + '-transform_vsh.dksh',
  input : files(src / 'graphics/deko3d/deko_examples/source/transform_vsh.glsl'),
  output : name + '-transform_vsh.dksh',
  command : [uam_prog, '-s', 'vert', '-o', '@OUTPUT@', '@INPUT@'],
  build_by_default : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew', '--romfs', src / 'graphics/deko3d/deko_examples/romfs'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── graphics/opengl/dynamic_resolution ────────────────────────────────────
name = 'graphics-opengl-dynamic_resolution'
sources = files(
  src / 'graphics/opengl/dynamic_resolution/source/main.cpp',
  src / 'graphics/opengl/dynamic_resolution/source/lenny.c',
)
elf = executable(name + '.elf', sources,
                 dependencies : [nx_dep, egl_dep, glapi_dep, drm_nouveau_dep, glad_dep],
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── graphics/opengl/es2gears ──────────────────────────────────────────────
name = 'graphics-opengl-es2gears'
elf = executable(name + '.elf', files(src / 'graphics/opengl/es2gears/source/main.c'),
                 dependencies : [nx_dep, egl_dep, glapi_dep, glesv2_dep, drm_nouveau_dep],
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── graphics/opengl/gpu_console ───────────────────────────────────────────
name = 'graphics-opengl-gpu_console'
sources = files(
  src / 'graphics/opengl/gpu_console/source/main.c',
  src / 'graphics/opengl/gpu_console/source/gpu_console.cpp',
)
elf = executable(name + '.elf', sources,
                 dependencies : [nx_dep, egl_dep, glapi_dep, drm_nouveau_dep, glad_dep],
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── graphics/opengl/lenny ─────────────────────────────────────────────────
name = 'graphics-opengl-lenny'
sources = files(
  src / 'graphics/opengl/lenny/source/main.cpp',
  src / 'graphics/opengl/lenny/source/lenny.c',
)
elf = executable(name + '.elf', sources,
                 dependencies : [nx_dep, egl_dep, glapi_dep, drm_nouveau_dep, glad_dep],
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── graphics/opengl/simple_triangle ───────────────────────────────────────
name = 'graphics-opengl-simple_triangle'
elf = executable(name + '.elf', files(src / 'graphics/opengl/simple_triangle/source/main.cpp'),
                 dependencies : [nx_dep, egl_dep, glapi_dep, drm_nouveau_dep, glad_dep],
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── graphics/opengl/textured_cube ─────────────────────────────────────────
name = 'graphics-opengl-textured_cube'
sources = files(
  src / 'graphics/opengl/textured_cube/source/main.cpp',
  src / 'graphics/opengl/textured_cube/source/stb_image.cpp',
)
elf = executable(name + '.elf', sources,
                 dependencies : [nx_dep, egl_dep, glapi_dep, drm_nouveau_dep, glad_dep],
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── graphics/printing/hello-world ─────────────────────────────────────────
name = 'graphics-printing-hello_world'
elf = executable(name + '.elf', files(src / 'graphics/printing/hello-world/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── graphics/printing/vt52-demo ───────────────────────────────────────────
name = 'graphics-printing-vt52_demo'
elf = executable(name + '.elf', files(src / 'graphics/printing/vt52-demo/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── graphics/sdl2/sdl2-demo ───────────────────────────────────────────────
name = 'graphics-sdl2_demo'
elf = executable(name + '.elf', files(src / 'graphics/sdl2/sdl2-demo/source/main.c'),
                 dependencies : [nx_dep, sdl2_dep, sdl2_image_dep, sdl2_mixer_dep, sdl2_ttf_dep],
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'cartene1ns', '--romfs', src / 'graphics/sdl2/sdl2-demo/romfs'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── graphics/sdl2/sdl2-simple ─────────────────────────────────────────────
name = 'graphics-sdl2_simple'
elf = executable(name + '.elf', files(src / 'graphics/sdl2/sdl2-simple/source/main.cpp'),
                 dependencies : [nx_dep, sdl2_dep],
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── graphics/shared_font ──────────────────────────────────────────────────
name = 'graphics-shared_font'
elf = executable(name + '.elf', files(src / 'graphics/shared_font/source/main.c'),
                 dependencies : [nx_dep, freetype2_dep],
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── graphics/simplegfx ────────────────────────────────────────────────────
name = 'graphics-simplegfx'
elf = executable(name + '.elf', files(src / 'graphics/simplegfx/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── graphics/simplegfx_moviemaker ─────────────────────────────────────────
name = 'graphics-simplegfx_moviemaker'
elf = executable(name + '.elf', files(src / 'graphics/simplegfx_moviemaker/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── hid/abstracted-pad ────────────────────────────────────────────────────
name = 'hid-abstracted_pad'
elf = executable(name + '.elf', files(src / 'hid/abstracted-pad/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── hid/colors ────────────────────────────────────────────────────────────
name = 'hid-colors'
elf = executable(name + '.elf', files(src / 'hid/colors/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── hid/hdls ──────────────────────────────────────────────────────────────
name = 'hid-hdls'
elf = executable(name + '.elf', files(src / 'hid/hdls/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── hid/irsensor ──────────────────────────────────────────────────────────
name = 'hid-irsensor'
elf = executable(name + '.elf', files(src / 'hid/irsensor/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── hid/notification-led ──────────────────────────────────────────────────
name = 'hid-notification_led'
elf = executable(name + '.elf', files(src / 'hid/notification-led/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── hid/read-controls ─────────────────────────────────────────────────────
name = 'hid-read_controls'
elf = executable(name + '.elf', files(src / 'hid/read-controls/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── hid/ring-con ──────────────────────────────────────────────────────────
name = 'hid-ring_con'
elf = executable(name + '.elf', files(src / 'hid/ring-con/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── hid/sevensixaxis ──────────────────────────────────────────────────────
name = 'hid-sevensixaxis'
elf = executable(name + '.elf', files(src / 'hid/sevensixaxis/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── hid/sixaxis ───────────────────────────────────────────────────────────
name = 'hid-sixaxis'
elf = executable(name + '.elf', files(src / 'hid/sixaxis/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── hid/touch-screen ──────────────────────────────────────────────────────
name = 'hid-touch_screen'
elf = executable(name + '.elf', files(src / 'hid/touch-screen/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── hid/vibration ─────────────────────────────────────────────────────────
name = 'hid-vibration'
elf = executable(name + '.elf', files(src / 'hid/vibration/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── jit ───────────────────────────────────────────────────────────────────
name = 'jit'
elf = executable(name + '.elf', files(src / 'jit/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── misc/barrier ──────────────────────────────────────────────────────────
name = 'misc-barrier'
elf = executable(name + '.elf', files(src / 'misc/barrier/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── misc/user_events ──────────────────────────────────────────────────────
name = 'misc-user_events'
elf = executable(name + '.elf', files(src / 'misc/user_events/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── misc/user_timers ──────────────────────────────────────────────────────
name = 'misc-user_timers'
elf = executable(name + '.elf', files(src / 'misc/user_timers/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── network/curl ──────────────────────────────────────────────────────────
name = 'network-curl'
elf = executable(name + '.elf', files(src / 'network/curl/source/main.c'),
                 dependencies : [nx_dep, curl_dep],
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── network/ldn ───────────────────────────────────────────────────────────
name = 'network-ldn'
elf = executable(name + '.elf', files(src / 'network/ldn/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── network/lp2p ──────────────────────────────────────────────────────────
name = 'network-lp2p'
elf = executable(name + '.elf', files(src / 'network/lp2p/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── network/nxlink_stdio ──────────────────────────────────────────────────
name = 'network-nxlink_stdio'
elf = executable(name + '.elf', files(src / 'network/nxlink_stdio/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── nfc ───────────────────────────────────────────────────────────────────
name = 'nfc'
elf = executable(name + '.elf', files(src / 'nfc/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── power ─────────────────────────────────────────────────────────────────
name = 'power'
elf = executable(name + '.elf', files(src / 'power/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── settings/get_system_language ──────────────────────────────────────────
name = 'get_system_language'
elf = executable(name + '.elf', files(src / 'settings/get_system_language/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── time ──────────────────────────────────────────────────────────────────
name = 'time'
elf = executable(name + '.elf', files(src / 'time/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)

# ── usb/usbhs ─────────────────────────────────────────────────────────────
name = 'usbsh'
elf = executable(name + '.elf', files(src / 'usb/usbhs/source/main.c'),
                 dependencies : nx_dep,
                 c_args : c_opts, cpp_args : cpp_opts,
                 link_args : ld_args, pie : true)
custom_target(name + '.nro',
              command : bundle_base + ['--name', name, '--author', 'switchbrew'],
              input : elf, output : name + '.nro',
              build_by_default : true, depends : elf)
