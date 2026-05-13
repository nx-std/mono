//! USB device stack wire-layout types.

use static_assertions::const_assert_eq;

/// USB device information (VID/PID/BCD and string descriptors). Pre-5.0.0 only.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct UsbDsDeviceInfo {
    pub id_vendor: u16,
    pub id_product: u16,
    pub bcd_device: u16,
    pub manufacturer: [u8; 0x20],
    pub product: [u8; 0x20],
    pub serial_number: [u8; 0x20],
}

const_assert_eq!(size_of::<UsbDsDeviceInfo>(), 0x66);

/// A single entry in a USB report.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct UsbDsReportEntry {
    pub id: u32,
    pub requested_size: u32,
    pub transferred_size: u32,
    pub urb_status: u32,
}

const_assert_eq!(size_of::<UsbDsReportEntry>(), 0x10);

/// USB report data containing up to 8 entries.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct UsbDsReportData {
    pub report: [UsbDsReportEntry; 8],
    pub report_count: u32,
}

const_assert_eq!(size_of::<UsbDsReportData>(), 0x84);

/// Wire-layout input for PostBufferAsync commands.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct PostBufferIn {
    pub size: u32,
    pub _pad: u32,
    pub buffer: u64,
}

const_assert_eq!(size_of::<PostBufferIn>(), 0x10);

/// Wire-layout input for AppendConfigurationData (pre-11.0.0).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct AppendConfigDataLegacyIn {
    pub intf_num: u8,
    pub _pad: [u8; 3],
    pub speed: u32,
}

const_assert_eq!(size_of::<AppendConfigDataLegacyIn>(), 0x8);

/// USB device speed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum UsbDeviceSpeed {
    None = 0,
    Low = 1,
    Full = 2,
    High = 3,
    Super = 4,
}

/// USB device state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum UsbState {
    Detached = 0,
    Attached = 1,
    Powered = 2,
    Default = 3,
    Address = 4,
    Configured = 5,
    Suspended = 6,
}

/// USB complex ID for bind operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum UsbComplexId {
    Default = 0x2,
}

/// USB string descriptor (wire layout for AddUsbStringDescriptor).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct UsbStringDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: u8,
    pub w_data: [u16; 0x40],
}

const_assert_eq!(size_of::<UsbStringDescriptor>(), 0x82);
