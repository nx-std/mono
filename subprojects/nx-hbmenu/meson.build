project('nx-hbmenu', 'c',
    version : '3.6.0',
    meson_version : '>= 1.3.0',
    default_options : [
        'buildtype=debugoptimized', # -g -O2
        'warning_level=1', # -Wall
    ],
)

# Project metadata
name = 'hbmenu'
author = 'switchbrew'
version = meson.project_version()

# If this is not a release build, use the git tag as the version
if get_option('buildtype') != 'release'
    git_rev = run_command('git', '-C', meson.current_source_dir() / 'src', 'describe', '--dirty', '--always', check : true).stdout().strip()
    version += '-@0@'.format(git_rev)
endif

#---------------------------------------------------------------------------------
# Dependencies
#---------------------------------------------------------------------------------
deps_override_link_args = []

## Required dependencies
# libnx
if get_option('use_libnx_dkp').enabled()
    nx_proj = subproject('libnx-dkp')
else
    nx_proj = subproject('libnx')
endif

nx_dep = nx_proj.get_variable('nx_dep')
nx_switch_specs = nx_proj.get_variable('nx_switch_specs')
nx_default_icon = nx_proj.get_variable('nx_default_icon')

# libdeko3d
deko3d_proj = subproject('libdeko3d-dkp')
deko3d_dep = deko3d_proj.get_variable('deko3d_dep')

# freetype2
freetype2_dep = dependency('freetype2', method : 'pkg-config', static : true, required : true)

# libphysfs
physfs_dep = dependency('physfs', method : 'pkg-config', static : true, required : true)

# libconfig
config_dep = dependency('libconfig', method : 'pkg-config', static : true, required : true)

# libturbojpeg
turbojpeg_dep = dependency('libturbojpeg', method : 'pkg-config', static : true, required : true)

# libpng16
png_dep = dependency('libpng16', method : 'pkg-config', static : true, required : true)

deps = [
    nx_dep,
    deko3d_dep,
    freetype2_dep,
    physfs_dep,
    config_dep,
    turbojpeg_dep,
]

#---------------------------------------------------------------------------------
# Pre-processing and assets
#---------------------------------------------------------------------------------
## RomFS (assets.zip)
zip_prog = find_program('zip', required : true)

romfs_assets = files(
    'src/assets/airplane_icon.bin',
    'src/assets/battery_icon.bin',
    'src/assets/charging_icon.bin',
    'src/assets/eth_icon.bin',
    'src/assets/eth_none_icon.bin',
    'src/assets/folder_icon.bin',
    'src/assets/hbmenu_logo_dark.bin',
    'src/assets/hbmenu_logo_light.bin',
    'src/assets/invalid_icon.bin',
    'src/assets/theme_icon_dark.bin',
    'src/assets/theme_icon_light.bin',
    'src/assets/wifi1_icon.bin',
    'src/assets/wifi2_icon.bin',
    'src/assets/wifi3_icon.bin',
    'src/assets/wifi_none_icon.bin',
)

# Create the output romfs directory
romfs_dir_tgt = custom_target('hbmenu-romfs-outdir',
    input : romfs_assets,
    output : 'romfs_dir_marker',
    command : ['mkdir', '-p', '@OUTDIR@/romfs'],
    build_by_default : true,
)

# Zip all assets into a single file named 'assets.zip',
# and place them within the 'romfs' directory
romfs_tgt = custom_target('hbmenu-romfs',
    input : romfs_assets,
    output : 'assets.zip',
    command : [zip_prog, '-j', '@OUTDIR@/romfs/assets.zip', '@INPUT@'],
    build_by_default : true,
    depends : romfs_dir_tgt,
)

# Romfs directory
romfs = meson.current_build_dir() / 'romfs'

#---------------------------------------------------------------------------------
# ELF
#---------------------------------------------------------------------------------
# Source files
c_src = files(
    'src/common/assets.c',
    'src/common/font.c',
    'src/common/language.c',
    'src/common/launch.c',
    'src/common/math.c',
    'src/common/menu.c',
    'src/common/menu-entry.c',
    'src/common/menu-list.c',
    'src/common/message-box.c',
    'src/common/netloader.c',
    'src/common/status.c',
    'src/common/text.c',
    'src/common/theme.c',
    'src/common/ui.c',
    'src/common/worker.c',
    'src/nx_main/main.c',
    'src/nx_main/nx_audio.c',
    'src/nx_main/nx_graphics.c',
    'src/nx_main/nx_launch.c',
    'src/nx_main/nx_netstatus.c',
    'src/nx_main/nx_power.c',
    'src/nx_main/nx_thermalstatus.c',
    'src/nx_main/nx_touch.c',
    'src/nx_main/loaders/builtin.c',
)

# Compiler (and linker) options
arch_opts = ['-march=armv8-a', '-mtune=cortex-a57', '-mtp=soft', '-fPIE']
c_opts = ['-ffunction-sections'] + arch_opts + ['-D__SWITCH__', '-DVERSION="v@0@"'.format(version)]
cpp_opts = c_opts + ['-fno-rtti', '-fno-exceptions']
ld_flags = ['-specs=@0@'.format(nx_switch_specs), '-g'] + arch_opts + ['-Wl,-Map,@0@/@1@.map'.format(meson.current_build_dir(), name)]

# Compile the ELF file
elf = executable(
    '@0@.elf'.format(name),
    c_src,
    dependencies : deps,
    c_args : c_opts,
    cpp_args : cpp_opts,
    link_args : ld_flags,
)

# Generate the *.lst file
nm = find_program('nm', required : true)
elf_lst = custom_target('@0@.lst'.format(name),
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
                  depends : romfs_tgt,
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
    elif fs.is_file('src/icon.jpg') # icon.jpg
        icon = 'src/icon.jpg'
    elif fs.is_file('src/@0@.jpg'.format(name)) # {name}.jpg
        icon = 'src/@0@.jpg'.format(name)
    elif fs.is_file(nx_default_icon) # libnx's default icon
        icon = nx_default_icon
    else
        error('No icon file found')
    endif
    bundle_opts += ['--icon', files(icon)]

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
                  depends : romfs_tgt,
                  build_by_default : true)
else
    error('Unsupported bundle type: @0@'.format(bundle_type))
endif
