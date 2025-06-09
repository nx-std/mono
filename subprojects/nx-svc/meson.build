project('nx-svc', 'c', version : '0.1.0', meson_version : '>= 1.3.0')

#---------------------------------------------------------------------------------
# C wrapper
#---------------------------------------------------------------------------------
# Source files
inc = include_directories('wrapper/include')

c_src = files('wrapper/source/wrapper.c')

# Compiler options should match the main libnx library
arch_opts = ['-march=armv8-a+crc+crypto', '-mtune=cortex-a57', '-mtp=soft', '-fPIC', '-ftls-model=local-exec']
c_opts = ['-g', '-ffunction-sections', '-fdata-sections'] + arch_opts
c_opts += ['-D__SWITCH__']

nx_svc_wrapper_lib = static_library('nx_svc_wrapper',
                                    c_src,
                                    include_directories : inc,
                                    c_args : c_opts)

#---------------------------------------------------------------------------------
# Static library (Rust)
#---------------------------------------------------------------------------------
cargo = find_program('cargo', required : true)

nx_svc_tgt = custom_target(
    'nx-svc',
    command : [
        cargo, 'build',
        '--package', meson.project_name(),
        '--profile', get_option('buildtype') == 'release' ? 'release' : 'dev',
        '--target-dir', '@PRIVATE_DIR@',
        '--artifact-dir', '@OUTDIR@',
    ],
    output : 'libnx_svc.a',
    build_by_default : true,
    build_always_stale: true,
)

nx_svc_dep = declare_dependency(
    include_directories : inc,
    link_with : [nx_svc_tgt[0], nx_svc_wrapper_lib],
)
