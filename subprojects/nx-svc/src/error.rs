//! Error codes for the SVC.
//!
//! This module contains the error codes for the SVC.

/// Identifies which module caused an error.
///
/// Note that error codes can propagate through a call chain, so this may not always
/// correspond to the module containing the API call that returned the error.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u32)]
pub enum Module {
    /// SVC
    Kernel = 1,
    FS = 2,
    /// Used for Memory, Thread, Mutex, Nvidia, etc.
    OS = 3,
    HTCS = 4,
    NCM = 5,
    DD = 6,
    LR = 8,
    Loader = 9,
    CMIF = 10,
    HIPC = 11,
    TMA = 12,
    DMNT = 13,
    GDS = 14,
    PM = 15,
    NS = 16,
    BSDSockets = 17,
    HTC = 18,
    TSC = 19,
    NCMContent = 20,
    SM = 21,
    RO = 22,
    GC = 23,
    SDMMC = 24,
    OVLN = 25,
    SPL = 26,
    Socket = 27,
    HTCLOW = 29,
    DDSF = 30,
    HTCFS = 31,
    Async = 32,
    Util = 33,
    TIPC = 35,
    ANIF = 37,
    ETHC = 100,
    I2C = 101,
    GPIO = 102,
    UART = 103,
    CPAD = 104,
    Settings = 105,
    FTM = 106,
    WLAN = 107,
    XCD = 108,
    TMP451 = 109,
    NIFM = 110,
    HwOpus = 111,
    LSM6DS3 = 112,
    Bluetooth = 113,
    VI = 114,
    NFP = 115,
    Time = 116,
    FGM = 117,
    OE = 118,
    BH1730FVC = 119,
    PCIe = 120,
    Friends = 121,
    BCAT = 122,
    SSLSrv = 123,
    Account = 124,
    News = 125,
    Mii = 126,
    NFC = 127,
    AM = 128,
    PlayReport = 129,
    AHID = 130,
    Qlaunch = 132,
    PCV = 133,
    USBPD = 134,
    BPC = 135,
    PSM = 136,
    NIM = 137,
    PSC = 138,
    TC = 139,
    USB = 140,
    NSD = 141,
    PCTL = 142,
    BTM = 143,
    LA = 144,
    ETicket = 145,
    NGC = 146,
    ERPT = 147,
    APM = 148,
    CEC = 149,
    Profiler = 150,
    ErrorUpload = 151,
    LIDBE = 152,
    Audio = 153,
    NPNS = 154,
    NPNSHTTPSTREAM = 155,
    ARP = 157,
    SWKBD = 158,
    BOOT = 159,
    NetDiag = 160,
    NFCMifare = 161,
    UserlandAssert = 162,
    Fatal = 163,
    NIMShop = 164,
    SPSM = 165,
    BGTC = 167,
    UserlandCrash = 168,
    SASBUS = 169,
    PI = 170,
    AudioCtrl = 172,
    LBL = 173,
    JIT = 175,
    HDCP = 176,
    OMM = 177,
    PDM = 178,
    OLSC = 179,
    SREPO = 180,
    Dauth = 181,
    STDFU = 182,
    DBG = 183,
    DHCPS = 186,
    SPI = 187,
    AVM = 188,
    PWM = 189,
    RTC = 191,
    Regulator = 192,
    LED = 193,
    SIO = 195,
    PCM = 196,
    CLKRST = 197,
    POWCTL = 198,
    AudioOld = 201,
    HID = 202,
    LDN = 203,
    CS = 204,
    Irsensor = 205,
    Capture = 206,
    Manu = 208,
    ATK = 209,
    WEB = 210,
    LCS = 211,
    GRC = 212,
    Repair = 213,
    Album = 214,
    RID = 215,
    Migration = 216,
    MigrationLdcServ = 217,
    HIDBUS = 218,
    ENS = 219,
    WebSocket = 223,
    DCDMTP = 227,
    PGL = 228,
    Notification = 229,
    INS = 230,
    LP2P = 231,
    RCD = 232,
    LCM40607 = 233,
    PRC = 235,
    TMAHTC = 237,
    ECTX = 238,
    MNPP = 239,
    HSHL = 240,
    CAPMTP = 242,
    DP2HDMI = 244,
    Cradle = 245,
    SProfile = 246,
    NDRM = 250,
    TSPM = 499,
    DevMenu = 500,
    GeneralWebApplet = 800,
    WifiWebAuthApplet = 809,
    WhitelistedApplet = 810,
    ShopN = 811,
}

/// Error description types
pub type Description = u32;

/// Converts an error description type into the value used in the error code.
pub trait IntoDescription {
    /// Converts the error description value into a `u32` .
    fn into_value(self) -> Description;
}

// Treat `u32` as a valid description type
impl IntoDescription for u32 {
    fn into_value(self) -> Description {
        self
    }
}

/// Raw error code type
pub type ResultCode = u32;

/// Converts an error into the raw result code a C caller receives.
///
/// This is the `nx-svc` crate family's conversion contract. Every error type
/// this crate declares implements it immediately after its own declaration, so
/// an error states how it renders as a result code exactly once instead of
/// once per FFI boundary that returns it.
///
/// Each crate family owns one such trait, because each answers a different
/// question. This one answers "which kernel result code describes this
/// failure" - the codes are the kernel's own, so every mapping here either
/// names a [`KernelError`] or forwards a code the kernel already produced. A
/// family layered on top of the kernel (the Service Framework, the runtime)
/// has its own vocabulary and its own fallback for failures the kernel never
/// saw, so it declares its own trait rather than widening this one.
///
/// A conversion that crosses families imports the other family's trait as `_`,
/// so the receiver selects the vocabulary and neither trait claims the name.
///
/// The trait is sealed. An error declared outside `nx-svc` belongs to another
/// family and its failures are not kernel failures, so implementing this trait
/// for one would put a code a caller decodes as a kernel result where no kernel
/// ever produced it. Such an error declares its family's own trait instead, as
/// `nx_sys_thread::error::ToResultCode` does.
pub trait ToResultCode: core::error::Error + _sealed::Sealed {
    /// Converts the error into a raw result code.
    fn to_rc(self) -> ResultCode;
}

/// Error codes for kernel operations
///
/// This is an enum of all the known error codes returned by the kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[repr(u32)]
pub enum KernelError {
    #[error("Out of sessions")]
    OutOfSessions = 7,
    #[error("Invalid argument")]
    InvalidArgument = 14,
    #[error("Not implemented")]
    NotImplemented = 33,
    #[error("No synchronization object")]
    NoSynchronizationObject = 57,
    #[error("Termination requested")]
    TerminationRequested = 59,
    #[error("Invalid size")]
    InvalidSize = 101,
    #[error("Invalid address")]
    InvalidAddress = 102,
    #[error("Out of resource")]
    OutOfResource = 103,
    #[error("Out of memory")]
    OutOfMemory = 104,
    #[error("Out of handles")]
    OutOfHandles = 105,
    #[error("Invalid current memory")]
    InvalidCurrentMemory = 106,
    #[error("Invalid new memory permission")]
    InvalidNewMemoryPermission = 108,
    #[error("Invalid memory region")]
    InvalidMemoryRegion = 110,
    #[error("Invalid priority")]
    InvalidPriority = 112,
    #[error("Invalid core id")]
    InvalidCoreId = 113,
    #[error("Invalid handle")]
    InvalidHandle = 114,
    #[error("Invalid pointer")]
    InvalidPointer = 115,
    #[error("Invalid combination")]
    InvalidCombination = 116,
    #[error("Timed out")]
    TimedOut = 117,
    #[error("Cancelled")]
    Cancelled = 118,
    #[error("Out of range")]
    OutOfRange = 119,
    #[error("Invalid enum value")]
    InvalidEnumValue = 120,
    #[error("Not found")]
    NotFound = 121,
    #[error("Busy")]
    Busy = 122,
    #[error("Session closed")]
    SessionClosed = 123,
    #[error("Invalid state")]
    InvalidState = 125,
    #[error("Reserved used")]
    ReservedUsed = 126,
    #[error("Port closed")]
    PortClosed = 131,
    #[error("Limit reached")]
    LimitReached = 132,
    #[error("Receive list broken")]
    ReceiveListBroken = 258,
    #[error("Out of address space")]
    OutOfAddressSpace = 259,
    #[error("Message too large")]
    MessageTooLarge = 260,
    #[error("Invalid id")]
    InvalidId = 519,
}

impl PartialEq<u32> for KernelError {
    /// Compares the error code with a raw description value.
    fn eq(&self, other: &u32) -> bool {
        *self as u32 == *other
    }
}

impl PartialEq<KernelError> for u32 {
    /// Compares the error code with a raw description value.
    fn eq(&self, other: &KernelError) -> bool {
        *self == *other as u32
    }
}

impl IntoDescription for KernelError {
    fn into_value(self) -> u32 {
        self as u32
    }
}

impl ToResultCode for KernelError {
    fn to_rc(self) -> ResultCode {
        pack(Module::Kernel, self.into_value())
    }
}

impl _sealed::Sealed for KernelError {}

/// Packs a module and a description into the raw result-code encoding.
///
/// The description occupies bits 9..22 and the module the low 9 bits, which is
/// the layout every Horizon OS result code uses.
const fn pack(module: Module, description: Description) -> ResultCode {
    (module as u32) | ((description & 0x1FFF) << 9)
}

pub(crate) mod _sealed {
    /// Restricts [`ToResultCode`](super::ToResultCode) to this crate's error
    /// types. Implemented immediately after every `ToResultCode` impl.
    pub trait Sealed {}
}
