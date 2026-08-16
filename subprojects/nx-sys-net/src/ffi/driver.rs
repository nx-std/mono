//! Bringing the socket driver up and down, from C.
//!
//! The C surface takes its configuration as a struct of plain integers and reports failure as a
//! Horizon result rather than an `errno`, because these run before there is a socket to have
//! failed. That is the only place in this module tree where a result code is a return value rather
//! than something left in a thread-local.

use nx_service_bsd::{
    BsdConfig,
    BsdServiceType,
    BufferEfficiency,
    ConfigVersion,
    ConnectError,
    ConnectOptions,
    SessionCount,
};

use super::errno;
use crate::{
    driver::{
        self,
        InitializeError,
    },
    session::ConnectFailed,
};

/// The C configuration struct, as a caller fills it in.
///
/// A separate type from [`ConnectOptions`] because it is a different thing: this is the untrusted
/// C input, whose integers may be zero, out of range, or nonsense, and [`ConnectOptions`] is what
/// this becomes once each of those has been ruled out.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SocketInitConfig {
    /// Initial TCP transmit buffer size.
    pub tcp_tx_buf_size: u32,
    /// Initial TCP receive buffer size.
    pub tcp_rx_buf_size: u32,
    /// Maximum TCP transmit buffer size; `0` pins it to the initial size.
    pub tcp_tx_buf_max_size: u32,
    /// Maximum TCP receive buffer size; `0` pins it to the initial size.
    pub tcp_rx_buf_max_size: u32,
    /// UDP transmit buffer size.
    pub udp_tx_buf_size: u32,
    /// UDP receive buffer size.
    pub udp_rx_buf_size: u32,
    /// How many buffers the service keeps per socket.
    pub sb_efficiency: u32,
    /// How many IPC sessions to hold open.
    pub num_bsd_sessions: u32,
    /// Which service variant to look up.
    pub bsd_service_type: u32,
}

/// The configuration the C driver hands out when a caller has no tuning need of its own.
///
/// Public because the runtime owns `socketInitialize` and so is what substitutes this for a null
/// configuration.
pub static DEFAULT_INIT_CONFIG: SocketInitConfig = SocketInitConfig {
    tcp_tx_buf_size: 0x8000,
    tcp_rx_buf_size: 0x10000,
    tcp_tx_buf_max_size: 0x40000,
    tcp_rx_buf_max_size: 0x40000,
    udp_tx_buf_size: 0x2400,
    udp_rx_buf_size: 0xA500,
    sb_efficiency: 4,
    num_bsd_sessions: 3,
    bsd_service_type: SERVICE_TYPE_USER,
};

/// Selects `bsd:u`.
const SERVICE_TYPE_USER: u32 = 1 << 0;
/// Selects `bsd:s`.
const SERVICE_TYPE_SYSTEM: u32 = 1 << 1;

/// Result module the C driver reports its own failures under.
const MODULE_LIBNX: u32 = 345;

/// `LibnxError_AlreadyInitialized`, as `libnx/include/switch/result.h` numbers it.
const ERROR_ALREADY_INITIALIZED: u32 = 7;
/// `LibnxError_BadInput`.
const ERROR_BAD_INPUT: u32 = 11;
/// `LibnxError_TooManyDevOpTabs`.
const ERROR_TOO_MANY_DEVOPTABS: u32 = 39;

/// Where the per-stage connect failures start.
///
/// The C driver answers a failed connect with whatever result the service
/// returned, because its own client reports one. This crate's client reports a
/// typed error naming the stage instead, and most of those stages carry no
/// result code to pass on — so the stage itself is what gets reported.
///
/// The base sits well above the last value `libnx`'s own enum defines (49), so
/// a reader decoding one of these against that enum finds nothing rather than
/// finding the wrong name. That is not a hypothetical: reading a made-up value
/// back as `LibnxError_AppletCmdidNotFound` is how these constants were found
/// to be wrong in the first place.
const ERROR_CONNECT_BASE: u32 = 200;

/// The runtime holds no service manager session to acquire the service over.
const ERROR_CONNECT_NO_SM: u32 = ERROR_CONNECT_BASE;
/// Neither `bsd:s` nor `bsd:u` could be acquired from the service manager.
const ERROR_CONNECT_GET_SERVICE: u32 = ERROR_CONNECT_BASE + 1;
/// The monitor session could not be acquired.
const ERROR_CONNECT_GET_MONITOR: u32 = ERROR_CONNECT_BASE + 2;
/// The transfer memory the service requires could not be created.
const ERROR_CONNECT_TRANSFER_MEMORY: u32 = ERROR_CONNECT_BASE + 3;
/// `RegisterClient` was rejected.
const ERROR_CONNECT_REGISTER_CLIENT: u32 = ERROR_CONNECT_BASE + 4;
/// `StartMonitoring` was rejected.
const ERROR_CONNECT_START_MONITORING: u32 = ERROR_CONNECT_BASE + 5;
/// The local transfer-memory handle could not be closed after registering.
const ERROR_CONNECT_CLOSE_TMEM: u32 = ERROR_CONNECT_BASE + 6;
/// A session could not be cloned to fill the pool.
const ERROR_CONNECT_CLONE_SESSION: u32 = ERROR_CONNECT_BASE + 7;
/// A connection was already established.
const ERROR_CONNECT_ALREADY: u32 = ERROR_CONNECT_BASE + 8;

/// Builds a Horizon result out of the module and description that name a failure.
const fn make_result(module: u32, description: u32) -> u32 {
    (module & 0x1FF) | ((description & 0x1FFF) << 9)
}

/// Returns the default socket driver configuration.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_sys_net__socketGetDefaultInitConfig() -> *const SocketInitConfig {
    &raw const DEFAULT_INIT_CONFIG
}

/// The result reported when the runtime holds no service manager session.
pub const NO_SERVICE_MANAGER: u32 = make_result(MODULE_LIBNX, ERROR_CONNECT_NO_SM);

/// Brings the socket driver up, reporting the Horizon result the C caller expects.
///
/// This is the body of `socketInitialize` without the symbol. The symbol itself cannot live here:
/// its C contract is that the interface revision follows the running firmware, and the firmware
/// version is held by the runtime, which a crate at this level may not depend on. So the runtime
/// owns the entry point and the ladder that picks `version`, and this is what it calls once the
/// choice has been made. See the crate documentation.
pub fn initialize(
    sm: &nx_service_sm::SmService,
    config: &SocketInitConfig,
    version: ConfigVersion,
) -> u32 {
    let Some(opts) = to_connect_options(config, version) else {
        return make_result(MODULE_LIBNX, ERROR_BAD_INPUT);
    };

    match driver::initialize(sm, &opts) {
        Ok(()) => {
            // A result left by an earlier session describes nothing about this one.
            errno::clear_last_result();
            0
        }
        Err(InitializeError::AlreadyInitialized) => {
            make_result(MODULE_LIBNX, ERROR_ALREADY_INITIALIZED)
        }
        Err(InitializeError::Register(_)) => make_result(MODULE_LIBNX, ERROR_TOO_MANY_DEVOPTABS),
        Err(InitializeError::Connect(err)) => make_result(MODULE_LIBNX, connect_description(&err)),
    }
}

/// Names the stage a failed connect stopped at.
///
/// One description per stage, because "the socket driver did not come up" is
/// not something a caller can act on: which of eight steps failed is what says
/// whether the service is missing, the memory could not be reserved, or the
/// configuration was refused.
fn connect_description(err: &ConnectFailed) -> u32 {
    match err {
        ConnectFailed::AlreadyConnected => ERROR_CONNECT_ALREADY,
        ConnectFailed::Connect(err) => match err {
            ConnectError::GetService(_) => ERROR_CONNECT_GET_SERVICE,
            ConnectError::GetMonitorService(_) => ERROR_CONNECT_GET_MONITOR,
            ConnectError::CreateTransferMemory(_) => ERROR_CONNECT_TRANSFER_MEMORY,
            ConnectError::RegisterClient(_) => ERROR_CONNECT_REGISTER_CLIENT,
            ConnectError::StartMonitoring(_) => ERROR_CONNECT_START_MONITORING,
            ConnectError::CloseTmemHandle(_) => ERROR_CONNECT_CLOSE_TMEM,
            ConnectError::CloneSession(_) => ERROR_CONNECT_CLONE_SESSION,
        },
    }
}

/// Takes the socket driver down.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_sys_net__socketExit() {
    driver::exit();
}

/// Returns the Horizon result the last failed socket call left on this thread.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_sys_net__socketGetLastResult() -> u32 {
    errno::last_result()
}

/// Validates a caller's configuration into the options the connect handshake takes.
///
/// Returns `None` when a field is outside what the service accepts. Zero is not such a value: the
/// C driver treats a zero session count or service type as "unspecified" and substitutes the
/// default, so that is what happens here too.
///
/// `version` is supplied rather than derived: see [`initialize`].
fn to_connect_options(config: &SocketInitConfig, version: ConfigVersion) -> Option<ConnectOptions> {
    let sb_efficiency = BufferEfficiency::try_from(config.sb_efficiency).ok()?;

    let num_sessions = match config.num_bsd_sessions {
        0 => SessionCount::DEFAULT,
        count => SessionCount::try_from(count).ok()?,
    };

    let service_type = match config.bsd_service_type {
        0 => BsdServiceType::User,
        SERVICE_TYPE_USER => BsdServiceType::User,
        SERVICE_TYPE_SYSTEM => BsdServiceType::System,
        both if both == SERVICE_TYPE_USER | SERVICE_TYPE_SYSTEM => BsdServiceType::Auto,
        _ => return None,
    };

    Some(ConnectOptions {
        service_type,
        config: BsdConfig {
            version,
            tcp_tx_buf_size: config.tcp_tx_buf_size,
            tcp_rx_buf_size: config.tcp_rx_buf_size,
            tcp_tx_buf_max_size: config.tcp_tx_buf_max_size,
            tcp_rx_buf_max_size: config.tcp_rx_buf_max_size,
            udp_tx_buf_size: config.udp_tx_buf_size,
            udp_rx_buf_size: config.udp_rx_buf_size,
            sb_efficiency,
            num_sessions,
        },
    })
}
