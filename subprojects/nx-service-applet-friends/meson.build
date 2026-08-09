project('nx-service-applet-friends', version : '0.1.0')

cargo = find_program('cargo', required : true)

#---------------------------------------------------------------------------------
# Dependencies
#---------------------------------------------------------------------------------
# Rust dependencies here are just informative so Meson can build the dependencies in the correct order
# nx-panic-handler
nx_panic_handler_proj = subproject('nx-panic-handler')
nx_panic_handler_dep = nx_panic_handler_proj.get_variable('nx_panic_handler_dep')

# nx-service-acc
nx_service_acc_proj = subproject('nx-service-acc')
nx_service_acc_dep = nx_service_acc_proj.get_variable('nx_service_acc_dep')

# nx-service-applet
nx_service_applet_proj = subproject('nx-service-applet')
nx_service_applet_dep = nx_service_applet_proj.get_variable('nx_service_applet_dep')

# nx-service-friends
nx_service_friends_proj = subproject('nx-service-friends')
nx_service_friends_dep = nx_service_friends_proj.get_variable('nx_service_friends_dep')

# nx-sf
nx_sf_proj = subproject('nx-sf')
nx_sf_dep = nx_sf_proj.get_variable('nx_sf_dep')

# Dependencies list
deps = [
    nx_panic_handler_dep,
    nx_service_acc_dep,
    nx_service_applet_dep,
    nx_service_friends_dep,
    nx_sf_dep,
]

#---------------------------------------------------------------------------------
# Static library
#---------------------------------------------------------------------------------
# Target
nx_service_applet_friends_tgt = custom_target(
    'nx-service-applet-friends',
    command : [
        cargo, 'build',
        '--package', meson.project_name(),
        '--profile', get_option('buildtype') == 'release' ? 'release' : 'dev',
        '--target-dir', meson.global_build_root() / 'cargo-target',
        '--artifact-dir', '@OUTDIR@',
    ],
    output : ['libnx_service_applet_friends.rlib'],
    console : true,
    build_by_default : true,
    build_always_stale : true,
)

#---------------------------------------------------------------------------------
# Dependency declaration
#---------------------------------------------------------------------------------
nx_service_applet_friends_dep = declare_dependency(
    sources : nx_service_applet_friends_tgt,
    dependencies : deps,
)
