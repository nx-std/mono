//! Public configuration types for the BSD socket service.

/// Which BSD socket service to connect to.
///
/// Mirrors libnx's `BsdServiceType` (`bsd.h`). `Auto` tries `bsd:s` first and
/// falls back to `bsd:u` — the same precedence libnx applies at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BsdServiceType {
    /// Try `bsd:s` first, fall back to `bsd:u` on failure.
    Auto,
    /// User-mode service `bsd:u`.
    User,
    /// System-mode service `bsd:s`.
    System,
}

/// BSD service initialization configuration.
///
/// Direct equivalent of libnx's `BsdInitConfig` plus the session-pool size.
/// All buffer sizes are in bytes and are passed verbatim to the service; the
/// crate computes the required transfer-memory size from them on the caller's
/// behalf.
#[derive(Debug, Clone)]
pub struct BsdConfig {
    /// Service interface version. `1` on `[2.0.0+]`, `2` on `[3.0.0+]`.
    pub version: u32,
    /// Initial TCP transmit buffer size, in bytes.
    pub tcp_tx_buf_size: u32,
    /// Initial TCP receive buffer size, in bytes.
    pub tcp_rx_buf_size: u32,
    /// Maximum TCP transmit buffer size. `0` pins the buffer to the initial size.
    pub tcp_tx_buf_max_size: u32,
    /// Maximum TCP receive buffer size. `0` pins the buffer to the initial size.
    pub tcp_rx_buf_max_size: u32,
    /// UDP transmit buffer size, in bytes.
    pub udp_tx_buf_size: u32,
    /// UDP receive buffer size, in bytes.
    pub udp_rx_buf_size: u32,
    /// Number of buffers per socket (`1..=8` per libnx).
    pub sb_efficiency: u32,
    /// Total IPC sessions held by the service. libnx defaults to `3`.
    ///
    /// Clamped to `1..=32` by the session pool — `0` is rounded up to `1`,
    /// values larger than `32` are saturated at `32` (the free-mask is a `u32`).
    pub num_sessions: u32,
}

impl BsdConfig {
    /// Returns the defaults from libnx's `bsdGetDefaultInitConfig`
    /// (`bsd.c::g_defaultBsdInitConfig`). Suitable starting point when callers
    /// have no specific tuning need.
    pub const fn default_libnx() -> Self {
        Self {
            version: 1,
            tcp_tx_buf_size: 0x8000,
            tcp_rx_buf_size: 0x10000,
            tcp_tx_buf_max_size: 0x40000,
            tcp_rx_buf_max_size: 0x40000,
            udp_tx_buf_size: 0x2400,
            udp_rx_buf_size: 0xA500,
            sb_efficiency: 4,
            num_sessions: 3,
        }
    }

    /// Computes the minimum transfer-memory size required for this config.
    ///
    /// Mirrors `_bsdGetTransferMemSizeForConfig` in `bsd.c`: a per-socket sum
    /// of the four max buffer sizes, rounded up to a page, times the buffer
    /// efficiency factor.
    pub(crate) const fn transfer_mem_size(&self) -> usize {
        let tcp_tx_max = if self.tcp_tx_buf_max_size != 0 {
            self.tcp_tx_buf_max_size
        } else {
            self.tcp_tx_buf_size
        };
        let tcp_rx_max = if self.tcp_rx_buf_max_size != 0 {
            self.tcp_rx_buf_max_size
        } else {
            self.tcp_rx_buf_size
        };

        let mut sum = tcp_tx_max
            .wrapping_add(tcp_rx_max)
            .wrapping_add(self.udp_tx_buf_size)
            .wrapping_add(self.udp_rx_buf_size);

        // Page round-up (0x1000 page).
        sum = sum.wrapping_add(0xFFF) & !0xFFF;

        (self.sb_efficiency as usize).wrapping_mul(sum as usize)
    }
}

/// Options consumed by [`crate::connect_with_options`].
#[derive(Debug, Clone)]
pub struct ConnectOptions {
    /// Which service variant to look up.
    pub service_type: BsdServiceType,
    /// Init parameters and session-pool size.
    pub config: BsdConfig,
}

impl ConnectOptions {
    /// Returns connect options using libnx defaults and the `Auto` service-type
    /// fallback (`bsd:s` then `bsd:u`).
    pub const fn default_libnx() -> Self {
        Self {
            service_type: BsdServiceType::Auto,
            config: BsdConfig::default_libnx(),
        }
    }
}
