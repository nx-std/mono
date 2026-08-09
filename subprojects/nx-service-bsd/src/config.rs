//! What a caller decides before the service is connected.
//!
//! Every value here is fixed once, at connect time, and read by the service
//! for the life of the session. Two of them are bounded and one is drawn from
//! a fixed set, so each is a type that cannot hold a value the service would
//! reject — a bound checked at construction is a bound nothing downstream has
//! to re-check, silently clamp, or document in prose.
//!
//! The ranges are the interface's own, not this crate's invention: the
//! service documents what it accepts, and each bound below is that.

/// Which BSD socket service to connect to.
///
/// `Auto` tries `bsd:s` first and falls back to `bsd:u`, which is the
/// precedence a client without a specific need wants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BsdServiceType {
    /// Try `bsd:s` first, fall back to `bsd:u` on failure.
    Auto,
    /// User-mode service `bsd:u`.
    User,
    /// System-mode service `bsd:s`.
    System,
}

/// The configuration revision a client declares to the service.
///
/// Two revisions have been observed in the wild and nothing branches on the
/// value, so an enum costs nothing: a firmware that introduces a third would
/// need a variant added, and that is an edit worth a human's attention rather
/// than an integer slipping through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigVersion {
    /// Observed on `[2.0.0+]`. The safe default.
    V1,
    /// Observed on `[3.0.0+]`.
    V2,
}

impl ConfigVersion {
    /// The value the service reads.
    pub(crate) const fn to_wire(self) -> u32 {
        match self {
            Self::V1 => 1,
            Self::V2 => 2,
        }
    }
}

/// How many buffers the service keeps per socket.
///
/// Multiplies the transfer memory the connect handshake has to allocate, which
/// is why the bound matters: a zero asks for a zero-sized allocation the kernel
/// rejects with an error naming nothing useful, and an unbounded value asks for
/// a reservation no process can satisfy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferEfficiency(u32);

impl BufferEfficiency {
    /// Lowest value the service accepts.
    pub const MIN: u32 = 1;
    /// Highest value the interface documents as standard.
    pub const MAX: u32 = 8;

    /// What the default configuration uses.
    pub const DEFAULT: Self = Self(4);

    /// The value the service reads.
    pub(crate) const fn to_wire(self) -> u32 {
        self.0
    }
}

impl TryFrom<u32> for BufferEfficiency {
    type Error = BufferEfficiencyError;

    /// # Errors
    ///
    /// [`BufferEfficiencyError`] when `count` falls outside
    /// [`MIN`](Self::MIN)`..=`[`MAX`](Self::MAX).
    fn try_from(count: u32) -> Result<Self, Self::Error> {
        match count {
            Self::MIN..=Self::MAX => Ok(Self(count)),
            other => Err(BufferEfficiencyError { count: other }),
        }
    }
}

/// Error returned when a buffer count falls outside the range the service
/// accepts.
///
/// Detected before the connect handshake begins, so no session was opened and
/// no transfer memory was allocated.
#[derive(Debug, thiserror::Error)]
#[error(
    "buffer count {count} is outside the {}..={} the service accepts",
    BufferEfficiency::MIN,
    BufferEfficiency::MAX
)]
pub struct BufferEfficiencyError {
    /// The count that was offered.
    pub count: u32,
}

/// How many IPC sessions the client holds open to the service.
///
/// One session serves one command at a time, so this is what decides how many
/// socket calls can be in flight at once. The ceiling is not the service's but
/// this crate's: the session pool tracks free slots in a `u32` bitset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionCount(u32);

impl SessionCount {
    /// Fewest sessions a pool can be built from.
    pub const MIN: u32 = 1;
    /// Most sessions the pool's free-mask can track.
    pub const MAX: u32 = 32;

    /// What the default configuration uses.
    pub const DEFAULT: Self = Self(3);

    /// The count, as a pool size.
    ///
    /// Exact rather than lossy: the value is bounded by [`MAX`](Self::MAX),
    /// which fits every `usize` this workspace targets.
    pub(crate) const fn to_len(self) -> usize {
        self.0 as usize
    }
}

impl TryFrom<u32> for SessionCount {
    type Error = SessionCountError;

    /// # Errors
    ///
    /// [`SessionCountError`] when `count` falls outside
    /// [`MIN`](Self::MIN)`..=`[`MAX`](Self::MAX). A caller that would once
    /// have been silently clamped is now told.
    fn try_from(count: u32) -> Result<Self, Self::Error> {
        match count {
            Self::MIN..=Self::MAX => Ok(Self(count)),
            other => Err(SessionCountError { count: other }),
        }
    }
}

/// Error returned when a session count falls outside the range the pool can
/// represent.
///
/// Detected before the connect handshake begins, so no session was opened.
#[derive(Debug, thiserror::Error)]
#[error(
    "session count {count} is outside the {}..={} the pool can hold",
    SessionCount::MIN,
    SessionCount::MAX
)]
pub struct SessionCountError {
    /// The count that was offered.
    pub count: u32,
}

/// BSD service initialization configuration.
///
/// The buffer sizes are byte counts passed verbatim to the service; the crate
/// computes the required transfer-memory size from them on the caller's
/// behalf.
#[derive(Debug, Clone)]
pub struct BsdConfig {
    /// The configuration revision this client declares.
    pub version: ConfigVersion,
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
    /// How many buffers the service keeps per socket.
    pub sb_efficiency: BufferEfficiency,
    /// How many IPC sessions to hold open.
    pub num_sessions: SessionCount,
}

impl BsdConfig {
    /// The configuration a client with no specific tuning need should send.
    ///
    /// These are the values the platform's own clients are observed to use.
    pub const DEFAULT: Self = Self::defaults();

    /// Body of [`Self::DEFAULT`], written as a `const fn` so the associated
    /// constant can be built from it.
    const fn defaults() -> Self {
        Self {
            version: ConfigVersion::V1,
            tcp_tx_buf_size: 0x8000,
            tcp_rx_buf_size: 0x10000,
            tcp_tx_buf_max_size: 0x40000,
            tcp_rx_buf_max_size: 0x40000,
            udp_tx_buf_size: 0x2400,
            udp_rx_buf_size: 0xA500,
            sb_efficiency: BufferEfficiency::DEFAULT,
            num_sessions: SessionCount::DEFAULT,
        }
    }

    /// Computes the minimum transfer-memory size required for this config.
    ///
    /// A per-socket sum of the four maximum buffer sizes, rounded up to a
    /// page, times the buffer count — which is what the service provisions the
    /// transfer memory against.
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

        (self.sb_efficiency.to_wire() as usize).wrapping_mul(sum as usize)
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
    /// The options a client with no specific need should connect with: the
    /// default configuration and the `Auto` service-type fallback.
    pub const DEFAULT: Self = Self {
        service_type: BsdServiceType::Auto,
        config: BsdConfig::DEFAULT,
    };
}
