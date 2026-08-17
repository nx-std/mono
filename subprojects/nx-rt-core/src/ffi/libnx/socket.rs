//! Socket driver initialization, and the network-interface descriptor hand-offs.
//!
//! The socket driver itself lives in [`nx_sys_net`], and every one of its C symbols is exported
//! from there: except these three.
//!
//! # Why these entry points are here
//!
//! `socketInitialize`'s C contract is that the interface revision it declares to the BSD service
//! follows the running firmware. That makes it the one socket call whose behaviour depends on the
//! system version, and the system version is held by this crate, which [`nx_sys_net`] may not
//! depend on. So the ladder from firmware to revision lives in [`version`] below, and the symbol
//! that needs it lives beside it, calling [`nx_sys_net`]'s Rust entry with the choice already made.
//!
//! This is the same arrangement the controller applet uses, and for the same reason.
//!
//! The two network-interface hand-offs are the glue tier of a two-tier surface, and this is not
//! their final home. Upstream splits them: `services/nifm.c` holds the command that takes a socket
//! descriptor the BSD service issued, and `runtime/devices/socket.c` holds the wrapper that takes a
//! *process* descriptor, translates it, and delegates. The workspace has a crate per service C
//! surface, and the wrapper belongs with the crate owning the surface it delegates to.
//!
//! They sit here because `nx-nifm` does not claim them yet. `nx-tls` is the worked example of where
//! they are going: it owns the TLS pair of these, sits *above* this crate so it reads the running
//! firmware itself, and needed no help from the runtime to do it. A version gate is a reason to
//! write a check, not a reason to strand a symbol away from its surface.
//!
//! What [`nx_sys_net`] owns either way is the map between the process's descriptors and the
//! service's, so that is all these call it for: [`nx_sys_net::ffi::descriptor::resolve`] on the
//! way in.
//!
//! ## Reading a service the C side owns
//!
//! Each takes a pointer to a libnx service struct. [`nx_sf::ffi::Service`] is that struct's shape,
//! and [`nx_sf::ffi::Service::as_domain_object`] addresses what it names without adopting it: the
//! C caller created the request and closes it, and nothing here may do either.
//!
//! A struct that names no object is one the C side never converted to a domain. libnx tolerates
//! that case because its own conversion is allowed to fail, and dispatches on the plain session
//! instead. This does not: [`nx_service_nifm`] models the interface as a domain object, so there is
//! nothing to dispatch a command through, and the call reports the failure an inactive service
//! would rather than guessing.
//!
//! ## What the firmware decides
//!
//! Both hand-offs arrived in `[3.0.0]` and do not exist below it. That is not a version an API
//! should branch on, so no API here holds one: [`offers_nifm_socket_descriptor`] answers whether
//! the command is there, in the one place a version is compared.

// TODO: move the two network-interface hand-offs to `nx-nifm`, which already claims the
//  `nifmRequest*` tier they delegate to. It needs the same shape `nx-tls` has: a dependency on
//  `nx-rt-core` so it reads the `[3.0.0]` gate itself.

use core::ffi::{
    c_int,
    c_long,
    c_void,
};

use nx_service_bsd::{
    ConfigVersion,
    SocketFd,
};
use nx_service_nifm::{
    NifmServiceType,
    connect_cmif,
    ffi::ForeignNifmRequest,
};
use nx_sf::{
    error::{
        LibnxError,
        ToResultCode as _,
        libnx_error,
    },
    ffi::Service,
    service::{
        DispatchError,
        ForeignDomainObject,
    },
};
use nx_sys_net::ffi::{
    descriptor,
    driver::{
        DEFAULT_INIT_CONFIG,
        SocketInitConfig,
    },
    errno,
};

use crate::{
    env::hos_version::{
        self,
        HosVersion,
    },
    services::sm,
};

/// The address a console answers with when it is on no network.
///
/// `INADDR_LOOPBACK`, in the host order the C caller reads it in.
const LOOPBACK_ADDRESS: c_long = 0x7F00_0001;

/// Brings the socket driver up.
///
/// A null `config` selects the driver's default, which is how `socketInitializeDefault` is
/// written.
///
/// # Safety
///
/// `config` must be null or point to a readable [`SocketInitConfig`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_socket_initialize(config: *const c_void) -> u32 {
    let config = if config.is_null() {
        DEFAULT_INIT_CONFIG
    } else {
        // SAFETY: the caller guarantees a readable configuration at a non-null pointer.
        unsafe { *config.cast::<SocketInitConfig>() }
    };

    // A process gets one service manager session and this crate holds it, so the driver is handed
    // the session rather than opening a second one: which does not get a second session, it
    // fails.
    let Ok(sm) = sm::session() else {
        return nx_sys_net::ffi::driver::NO_SERVICE_MANAGER;
    };

    nx_sys_net::ffi::driver::initialize(&sm, &config, version())
}

/// Picks the interface revision the running firmware introduced.
///
/// The service accepts any revision up to its own, so this is an upper bound rather than an exact
/// match: declaring less than the firmware supports still works, and declaring more does not.
fn version() -> ConfigVersion {
    let current = hos_version::get();

    if current >= HosVersion::new(16, 0, 0) {
        ConfigVersion::V9
    } else if current >= HosVersion::new(13, 0, 0) {
        ConfigVersion::V8
    } else if current >= HosVersion::new(9, 0, 0) {
        ConfigVersion::V7
    } else if current >= HosVersion::new(8, 0, 0) {
        ConfigVersion::V6
    } else if current >= HosVersion::new(6, 0, 0) {
        ConfigVersion::V5
    } else if current >= HosVersion::new(5, 0, 0) {
        ConfigVersion::V4
    } else if current >= HosVersion::new(4, 0, 0) {
        ConfigVersion::V3
    } else if current >= HosVersion::new(3, 0, 0) {
        ConfigVersion::V2
    } else {
        ConfigVersion::V1
    }
}

/// Registers a socket descriptor with a network-interface request.
///
/// # Safety
///
/// `request` must point to a readable libnx `NifmRequest`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_socket_nifm_request_register_socket_descriptor(
    request: *mut c_void,
    sockfd: c_int,
) -> c_int {
    // SAFETY: the caller guarantees a readable `NifmRequest` at `request`, whose first member is
    // the service struct this reads.
    let object = unsafe { domain_object_at(request) };
    nifm_socket_descriptor(
        object,
        sockfd,
        ForeignNifmRequest::register_socket_descriptor,
    )
}

/// Unregisters a socket descriptor from a network-interface request.
///
/// # Safety
///
/// `request` must point to a readable libnx `NifmRequest`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_socket_nifm_request_unregister_socket_descriptor(
    request: *mut c_void,
    sockfd: c_int,
) -> c_int {
    // SAFETY: the caller guarantees a readable `NifmRequest` at `request`, whose first member is
    // the service struct this reads.
    let object = unsafe { domain_object_at(request) };
    nifm_socket_descriptor(
        object,
        sockfd,
        ForeignNifmRequest::unregister_socket_descriptor,
    )
}

/// Reports the address the console is currently reachable at.
///
/// Corresponds to `gethostid()` in `sys/unistd.h`. A console with no network
/// answers the loopback address, which is what a caller with nothing better to
/// say reports as "not connected".
///
/// The session it asks through is opened and closed around the question. The
/// answer changes whenever the console joins or leaves a network, so there is
/// nothing here worth keeping between calls.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_gethostid() -> c_long {
    // Every failure below answers with loopback rather than reporting: the C
    // signature has no way to say "no answer", and a caller asking where it can
    // be reached is better served by the address that always works than by an
    // address that is not the console's.
    let Ok(sm) = sm::session() else {
        return LOOPBACK_ADDRESS;
    };

    let Ok(mut nifm) = connect_cmif(&sm, NifmServiceType::User) else {
        return LOOPBACK_ADDRESS;
    };

    // The command that opens the general-service sub-interface was renumbered
    // in HOS 3.0.0, so which one to send is a property of the running firmware,
    // which this crate holds.
    let opened = if hos_version::get() >= HosVersion::new(3, 0, 0) {
        nifm.open_general_service()
    } else {
        nifm.open_general_service_legacy()
    };
    if opened.is_err() {
        return LOOPBACK_ADDRESS;
    }

    match nifm.get_current_ip_address() {
        Ok(addr) => c_long::from(addr.as_u32()),
        Err(_) => LOOPBACK_ADDRESS,
    }
}

/// Runs one of the network-interface request's two socket-descriptor commands.
///
/// The pair differ only in which command they send: the firmware they need, the descriptor
/// translation and the way each failure is reported are the same, so they are written once here.
fn nifm_socket_descriptor(
    object: Option<ForeignDomainObject<'static>>,
    sockfd: c_int,
    command: impl FnOnce(&ForeignNifmRequest<'static>, SocketFd) -> Result<(), DispatchError>,
) -> c_int {
    // The guards run in the order the C driver applies them: the descriptor is resolved before the
    // service function is called, and that function tests the request before the firmware.
    let sock = match descriptor::resolve(sockfd) {
        Ok(sock) => sock,
        Err(number) => return errno::fail(number),
    };

    let Some(object) = object else {
        return errno::report_result(libnx_error(LibnxError::NotInitialized));
    };

    if !offers_nifm_socket_descriptor() {
        return errno::report_result(libnx_error(LibnxError::IncompatSysVer));
    }

    match command(&ForeignNifmRequest::new(object), sock) {
        Ok(()) => 0,
        Err(err) => errno::report_result(err.to_rc()),
    }
}

/// Whether the running firmware implements the network-interface hand-offs.
///
/// One resolver for the whole module, so a firmware version is compared in exactly one place and
/// every entry point below reads a named fact instead of a version. The commands are not optional
/// features of one interface: each was introduced by a firmware and simply does not exist before
/// it, so what a caller needs to know is whether the command is there, not which release it is on.
///
/// The version is a run-constant: the entry crate stores it once during environment setup and it
/// cannot change while the process lives: so this recomputes nothing that could have moved.
fn offers_nifm_socket_descriptor() -> bool {
    hos_version::get() >= HosVersion::new(3, 0, 0)
}

/// Reads the libnx service struct at `ptr` and addresses the object it names.
///
/// Returns `None` when the struct names no object, which is what a service the C side never
/// converted to a domain looks like. Both interfaces reached from here are modelled as domain
/// objects, so there is nothing to send a command through in that case.
///
/// # Safety
///
/// `ptr` must be null or point to a readable libnx service struct: which every type reached
/// through here begins with, so a pointer to one of those is a pointer to this.
unsafe fn domain_object_at(ptr: *mut c_void) -> Option<ForeignDomainObject<'static>> {
    if ptr.is_null() {
        return None;
    }

    // SAFETY: the caller guarantees a readable service struct at a non-null `ptr`.
    let service = unsafe { *ptr.cast::<Service>() };
    service.as_domain_object()
}
