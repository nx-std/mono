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

impl TryFrom<u32> for Module {
    type Error = UnknownModuleError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Kernel),
            2 => Ok(Self::FS),
            3 => Ok(Self::OS),
            4 => Ok(Self::HTCS),
            5 => Ok(Self::NCM),
            6 => Ok(Self::DD),
            8 => Ok(Self::LR),
            9 => Ok(Self::Loader),
            10 => Ok(Self::CMIF),
            11 => Ok(Self::HIPC),
            12 => Ok(Self::TMA),
            13 => Ok(Self::DMNT),
            14 => Ok(Self::GDS),
            15 => Ok(Self::PM),
            16 => Ok(Self::NS),
            17 => Ok(Self::BSDSockets),
            18 => Ok(Self::HTC),
            19 => Ok(Self::TSC),
            20 => Ok(Self::NCMContent),
            21 => Ok(Self::SM),
            22 => Ok(Self::RO),
            23 => Ok(Self::GC),
            24 => Ok(Self::SDMMC),
            25 => Ok(Self::OVLN),
            26 => Ok(Self::SPL),
            27 => Ok(Self::Socket),
            29 => Ok(Self::HTCLOW),
            30 => Ok(Self::DDSF),
            31 => Ok(Self::HTCFS),
            32 => Ok(Self::Async),
            33 => Ok(Self::Util),
            35 => Ok(Self::TIPC),
            37 => Ok(Self::ANIF),
            100 => Ok(Self::ETHC),
            101 => Ok(Self::I2C),
            102 => Ok(Self::GPIO),
            103 => Ok(Self::UART),
            104 => Ok(Self::CPAD),
            105 => Ok(Self::Settings),
            106 => Ok(Self::FTM),
            107 => Ok(Self::WLAN),
            108 => Ok(Self::XCD),
            109 => Ok(Self::TMP451),
            110 => Ok(Self::NIFM),
            111 => Ok(Self::HwOpus),
            112 => Ok(Self::LSM6DS3),
            113 => Ok(Self::Bluetooth),
            114 => Ok(Self::VI),
            115 => Ok(Self::NFP),
            116 => Ok(Self::Time),
            117 => Ok(Self::FGM),
            118 => Ok(Self::OE),
            119 => Ok(Self::BH1730FVC),
            120 => Ok(Self::PCIe),
            121 => Ok(Self::Friends),
            122 => Ok(Self::BCAT),
            123 => Ok(Self::SSLSrv),
            124 => Ok(Self::Account),
            125 => Ok(Self::News),
            126 => Ok(Self::Mii),
            127 => Ok(Self::NFC),
            128 => Ok(Self::AM),
            129 => Ok(Self::PlayReport),
            130 => Ok(Self::AHID),
            132 => Ok(Self::Qlaunch),
            133 => Ok(Self::PCV),
            134 => Ok(Self::USBPD),
            135 => Ok(Self::BPC),
            136 => Ok(Self::PSM),
            137 => Ok(Self::NIM),
            138 => Ok(Self::PSC),
            139 => Ok(Self::TC),
            140 => Ok(Self::USB),
            141 => Ok(Self::NSD),
            142 => Ok(Self::PCTL),
            143 => Ok(Self::BTM),
            144 => Ok(Self::LA),
            145 => Ok(Self::ETicket),
            146 => Ok(Self::NGC),
            147 => Ok(Self::ERPT),
            148 => Ok(Self::APM),
            149 => Ok(Self::CEC),
            150 => Ok(Self::Profiler),
            151 => Ok(Self::ErrorUpload),
            152 => Ok(Self::LIDBE),
            153 => Ok(Self::Audio),
            154 => Ok(Self::NPNS),
            155 => Ok(Self::NPNSHTTPSTREAM),
            157 => Ok(Self::ARP),
            158 => Ok(Self::SWKBD),
            159 => Ok(Self::BOOT),
            160 => Ok(Self::NetDiag),
            161 => Ok(Self::NFCMifare),
            162 => Ok(Self::UserlandAssert),
            163 => Ok(Self::Fatal),
            164 => Ok(Self::NIMShop),
            165 => Ok(Self::SPSM),
            167 => Ok(Self::BGTC),
            168 => Ok(Self::UserlandCrash),
            169 => Ok(Self::SASBUS),
            170 => Ok(Self::PI),
            172 => Ok(Self::AudioCtrl),
            173 => Ok(Self::LBL),
            175 => Ok(Self::JIT),
            176 => Ok(Self::HDCP),
            177 => Ok(Self::OMM),
            178 => Ok(Self::PDM),
            179 => Ok(Self::OLSC),
            180 => Ok(Self::SREPO),
            181 => Ok(Self::Dauth),
            182 => Ok(Self::STDFU),
            183 => Ok(Self::DBG),
            186 => Ok(Self::DHCPS),
            187 => Ok(Self::SPI),
            188 => Ok(Self::AVM),
            189 => Ok(Self::PWM),
            191 => Ok(Self::RTC),
            192 => Ok(Self::Regulator),
            193 => Ok(Self::LED),
            195 => Ok(Self::SIO),
            196 => Ok(Self::PCM),
            197 => Ok(Self::CLKRST),
            198 => Ok(Self::POWCTL),
            201 => Ok(Self::AudioOld),
            202 => Ok(Self::HID),
            203 => Ok(Self::LDN),
            204 => Ok(Self::CS),
            205 => Ok(Self::Irsensor),
            206 => Ok(Self::Capture),
            208 => Ok(Self::Manu),
            209 => Ok(Self::ATK),
            210 => Ok(Self::WEB),
            211 => Ok(Self::LCS),
            212 => Ok(Self::GRC),
            213 => Ok(Self::Repair),
            214 => Ok(Self::Album),
            215 => Ok(Self::RID),
            216 => Ok(Self::Migration),
            217 => Ok(Self::MigrationLdcServ),
            218 => Ok(Self::HIDBUS),
            219 => Ok(Self::ENS),
            223 => Ok(Self::WebSocket),
            227 => Ok(Self::DCDMTP),
            228 => Ok(Self::PGL),
            229 => Ok(Self::Notification),
            230 => Ok(Self::INS),
            231 => Ok(Self::LP2P),
            232 => Ok(Self::RCD),
            233 => Ok(Self::LCM40607),
            235 => Ok(Self::PRC),
            237 => Ok(Self::TMAHTC),
            238 => Ok(Self::ECTX),
            239 => Ok(Self::MNPP),
            240 => Ok(Self::HSHL),
            242 => Ok(Self::CAPMTP),
            244 => Ok(Self::DP2HDMI),
            245 => Ok(Self::Cradle),
            246 => Ok(Self::SProfile),
            250 => Ok(Self::NDRM),
            499 => Ok(Self::TSPM),
            500 => Ok(Self::DevMenu),
            unknown => Err(UnknownModuleError(unknown)),
        }
    }
}

/// Errors returned when decoding the module field of a result code.
///
/// Occurs when the 9-bit module field names a module this build does not know:
/// a module added by a later firmware, or one of the four variants below whose
/// value does not fit the field at all.
///
/// The module field is 9 bits wide, so it cannot represent a value above 511.
/// These variants are therefore never produced by a decode, and a result code
/// packed from one of them carries its value truncated to the low 9 bits:
/// - [`Module::GeneralWebApplet`] (800)
/// - [`Module::WifiWebAuthApplet`] (809)
/// - [`Module::WhitelistedApplet`] (810)
/// - [`Module::ShopN`] (811)
#[derive(Debug, thiserror::Error)]
#[error("Unknown result code module {0}")]
pub struct UnknownModuleError(pub u32);

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

impl TryFrom<u32> for KernelError {
    type Error = UnknownKernelError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            7 => Ok(Self::OutOfSessions),
            14 => Ok(Self::InvalidArgument),
            33 => Ok(Self::NotImplemented),
            57 => Ok(Self::NoSynchronizationObject),
            59 => Ok(Self::TerminationRequested),
            101 => Ok(Self::InvalidSize),
            102 => Ok(Self::InvalidAddress),
            103 => Ok(Self::OutOfResource),
            104 => Ok(Self::OutOfMemory),
            105 => Ok(Self::OutOfHandles),
            106 => Ok(Self::InvalidCurrentMemory),
            108 => Ok(Self::InvalidNewMemoryPermission),
            110 => Ok(Self::InvalidMemoryRegion),
            112 => Ok(Self::InvalidPriority),
            113 => Ok(Self::InvalidCoreId),
            114 => Ok(Self::InvalidHandle),
            115 => Ok(Self::InvalidPointer),
            116 => Ok(Self::InvalidCombination),
            117 => Ok(Self::TimedOut),
            118 => Ok(Self::Cancelled),
            119 => Ok(Self::OutOfRange),
            120 => Ok(Self::InvalidEnumValue),
            121 => Ok(Self::NotFound),
            122 => Ok(Self::Busy),
            123 => Ok(Self::SessionClosed),
            125 => Ok(Self::InvalidState),
            126 => Ok(Self::ReservedUsed),
            131 => Ok(Self::PortClosed),
            132 => Ok(Self::LimitReached),
            258 => Ok(Self::ReceiveListBroken),
            259 => Ok(Self::OutOfAddressSpace),
            260 => Ok(Self::MessageTooLarge),
            519 => Ok(Self::InvalidId),
            unknown => Err(UnknownKernelError(unknown)),
        }
    }
}

/// Errors returned when decoding the description field of a kernel result code.
///
/// Occurs when the description names a failure this build does not know, which
/// happens on a firmware that added a code after these variants were written.
/// The raw description is preserved so a caller can still report it.
#[derive(Debug, thiserror::Error)]
#[error("Unknown kernel error description {0}")]
pub struct UnknownKernelError(pub u32);

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
        crate::result::raw::ResultCode::from_parts(Module::Kernel, self).to_raw()
    }
}

impl _sealed::Sealed for KernelError {}

pub(crate) mod _sealed {
    /// Restricts [`ToResultCode`](super::ToResultCode) to this crate's error
    /// types. Implemented immediately after every `ToResultCode` impl.
    pub trait Sealed {}
}
