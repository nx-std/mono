project('nx-svc', 'c', version : '0.1.0')

cargo = find_program('cargo', required : true)

## Static library
# Source files
inc = include_directories('include')

# Target
nx_svc_build = custom_target(
    'nx-svc',
    command : [cargo, 'build', '--release', '--target-dir', '@PRIVATE_DIR@', '--artifact-dir', '@OUTDIR@'],
    output : 'libnx_svc.a',
    build_always : true,
)

nx_svc_dep = declare_dependency(
    include_directories : inc,
    link_with : nx_svc_build[0],
)