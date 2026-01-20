//! NV service protocol constants and types.

use nx_sf::ServiceName;

/// Service name for nvdrv (Application).
pub const SERVICE_NAME_APPLICATION: ServiceName = ServiceName::new_truncate("nvdrv");

/// Service name for nvdrv:a (Applet).
pub const SERVICE_NAME_APPLET: ServiceName = ServiceName::new_truncate("nvdrv:a");

/// Service name for nvdrv:s (System).
pub const SERVICE_NAME_SYSTEM: ServiceName = ServiceName::new_truncate("nvdrv:s");

/// Service name for nvdrv:t (Factory).
pub const SERVICE_NAME_FACTORY: ServiceName = ServiceName::new_truncate("nvdrv:t");

/// INvDrvServices command IDs.
pub mod nv_cmds {
    /// Open a device by path.
    pub const OPEN: u32 = 0;

    /// Perform an ioctl operation.
    pub const IOCTL: u32 = 1;

    /// Close a device file descriptor.
    pub const CLOSE: u32 = 2;

    /// Initialize the service with transfer memory.
    pub const INITIALIZE: u32 = 3;

    /// Query an event handle for a device.
    pub const QUERY_EVENT: u32 = 4;

    /// Set client PID (with ARUID).
    pub const SET_CLIENT_PID: u32 = 8;

    /// Ioctl with extra input buffer (3.0.0+).
    pub const IOCTL2: u32 = 11;

    /// Ioctl with extra output buffer (3.0.0+).
    pub const IOCTL3: u32 = 12;
}

/// Ioctls that should use the cloned session for parallel execution.
///
/// These are high-frequency or potentially blocking operations that benefit
/// from being executed on a separate session to avoid contention.
pub const CLONE_SESSION_IOCTLS: &[u32] = &[
    0xC000_4402, // NVGPU_DBG_GPU_IOCTL_REG_OPS
    0xC000_471C, // NVGPU_GPU_IOCTL_GET_GPU_TIME
    0xC000_4808, // NVGPU_IOCTL_CHANNEL_SUBMIT_GPFIFO
    0xC000_0024, // NVHOST_IOCTL_CHANNEL_SUBMIT_EX
    0xC000_0025, // NVHOST_IOCTL_CHANNEL_MAP_CMD_BUFFER_EX
    0xC000_0026, // NVHOST_IOCTL_CHANNEL_UNMAP_CMD_BUFFER_EX
];

/// Full ioctl codes that should use the cloned session.
///
/// Unlike the masked values in [`CLONE_SESSION_IOCTLS`], these must match exactly.
pub const CLONE_SESSION_IOCTLS_EXACT: &[u32] = &[
    0xC018_481B, // NVGPU_IOCTL_CHANNEL_KICKOFF_PB
    0xC004_001C, // NVHOST_IOCTL_CTRL_EVENT_SIGNAL
    0xC010_001E, // NVHOST_IOCTL_CTRL_EVENT_WAIT_ASYNC
    0xC4C8_0203, // NVDISP_FLIP
    0x400C_060E, // NVSCHED_CTRL_PUT_CONDUCTOR_FLIP_FENCE
];

/// Mask for comparing ioctl request codes.
pub const IOCTL_MASK: u32 = 0xC000_FFFF;
