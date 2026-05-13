//! USB host stack wire-layout types.

use bitflags::bitflags;
use static_assertions::const_assert_eq;

// ---------------------------------------------------------------------------
// Standard USB descriptor types
// ---------------------------------------------------------------------------

/// USB endpoint descriptor (7 bytes, packed per USB spec).
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct UsbEndpointDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: u8,
    pub b_endpoint_address: u8,
    pub bm_attributes: u8,
    pub w_max_packet_size: u16,
    pub b_interval: u8,
}

const_assert_eq!(size_of::<UsbEndpointDescriptor>(), 0x7);

/// USB SuperSpeed endpoint companion descriptor (6 bytes).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct UsbSsEndpointCompanionDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: u8,
    pub b_max_burst: u8,
    pub bm_attributes: u8,
    pub w_bytes_per_interval: u16,
}

const_assert_eq!(size_of::<UsbSsEndpointCompanionDescriptor>(), 0x6);

/// USB interface descriptor (9 bytes).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct UsbInterfaceDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: u8,
    pub b_interface_number: u8,
    pub b_alternate_setting: u8,
    pub b_num_endpoints: u8,
    pub b_interface_class: u8,
    pub b_interface_sub_class: u8,
    pub b_interface_protocol: u8,
    pub i_interface: u8,
}

const_assert_eq!(size_of::<UsbInterfaceDescriptor>(), 0x9);

/// USB device descriptor (18 bytes).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct UsbDeviceDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: u8,
    pub bcd_usb: u16,
    pub b_device_class: u8,
    pub b_device_sub_class: u8,
    pub b_device_protocol: u8,
    pub b_max_packet_size0: u8,
    pub id_vendor: u16,
    pub id_product: u16,
    pub bcd_device: u16,
    pub i_manufacturer: u8,
    pub i_product: u8,
    pub i_serial_number: u8,
    pub b_num_configurations: u8,
}

const_assert_eq!(size_of::<UsbDeviceDescriptor>(), 0x12);

/// USB configuration descriptor (9 bytes, packed).
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct UsbConfigDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: u8,
    pub w_total_length: u16,
    pub b_num_interfaces: u8,
    pub b_configuration_value: u8,
    pub i_configuration: u8,
    pub bm_attributes: u8,
    pub max_power: u8,
}

const_assert_eq!(size_of::<UsbConfigDescriptor>(), 0x9);

// ---------------------------------------------------------------------------
// USB host stack types
// ---------------------------------------------------------------------------

bitflags! {
    /// Interface filter flags. When set, the corresponding descriptor field
    /// is compared during interface matching.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct UsbHsInterfaceFilterFlags: u16 {
        const ID_VENDOR = 1 << 0;
        const ID_PRODUCT = 1 << 1;
        /// 6.0.0+
        const BCD_DEVICE_MIN = 1 << 2;
        /// 6.0.0+
        const BCD_DEVICE_MAX = 1 << 3;
        /// 6.0.0+
        const B_DEVICE_CLASS = 1 << 4;
        /// 6.0.0+
        const B_DEVICE_SUB_CLASS = 1 << 5;
        /// 6.0.0+
        const B_DEVICE_PROTOCOL = 1 << 6;
        const B_INTERFACE_CLASS = 1 << 7;
        const B_INTERFACE_SUB_CLASS = 1 << 8;
        const B_INTERFACE_PROTOCOL = 1 << 9;
    }
}

/// Interface filtering struct. When a flag bit is set, the corresponding
/// descriptor field and struct field are compared; on mismatch the interface
/// is filtered out.
///
/// On 7.0.0+ the filter must be unique (not shared with other processes) and
/// `flags` must be non-zero.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct UsbHsInterfaceFilter {
    pub flags: u16,
    pub id_vendor: u16,
    pub id_product: u16,
    pub bcd_device_min: u16,
    pub bcd_device_max: u16,
    pub b_device_class: u8,
    pub b_device_sub_class: u8,
    pub b_device_protocol: u8,
    pub b_interface_class: u8,
    pub b_interface_sub_class: u8,
    pub b_interface_protocol: u8,
}

const_assert_eq!(size_of::<UsbHsInterfaceFilter>(), 0x10);

/// Interface information (packed). Contains the interface descriptor,
/// endpoint descriptors, and SuperSpeed endpoint companion descriptors.
///
/// The INPUT/OUTPUT endpoint descriptors were swapped at 8.0.0. This crate
/// does not perform the swap — callers targeting pre-8.0.0 must swap
/// input/output descriptor arrays themselves.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct UsbHsInterfaceInfo {
    pub id: i32,
    pub device_id_2: u32,
    pub unk_x8: u32,
    pub interface_desc: UsbInterfaceDescriptor,
    pub _pad_x15: [u8; 0x7],
    pub input_endpoint_descs: [UsbEndpointDescriptor; 15],
    pub _pad_x85: [u8; 0x7],
    pub output_endpoint_descs: [UsbEndpointDescriptor; 15],
    pub _pad_xf5: [u8; 0x6],
    pub input_ss_endpoint_companion_descs: [UsbSsEndpointCompanionDescriptor; 15],
    pub _pad_x155: [u8; 0x6],
    pub output_ss_endpoint_companion_descs: [UsbSsEndpointCompanionDescriptor; 15],
    pub _pad_x1b5: [u8; 0x3],
}

const_assert_eq!(size_of::<UsbHsInterfaceInfo>(), 0x1B8);

/// Full interface struct (packed). Each USB device has a separate
/// `UsbHsInterface` per interface.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct UsbHsInterface {
    pub inf: UsbHsInterfaceInfo,
    pub pathstr: [u8; 0x40],
    pub bus_id: u32,
    pub device_id: u32,
    pub device_desc: UsbDeviceDescriptor,
    pub config_desc: UsbConfigDescriptor,
    pub _pad_x21b: [u8; 0x5],
    pub timestamp: u64,
}

const_assert_eq!(size_of::<UsbHsInterface>(), 0x228);

/// Transfer report entry.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct UsbHsXferReport {
    pub xfer_id: u32,
    pub res: u32,
    pub requested_size: u32,
    pub transferred_size: u32,
    pub id: u64,
}

const_assert_eq!(size_of::<UsbHsXferReport>(), 0x18);

/// Ring buffer header for shared report ring.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct UsbHsRingHeader {
    pub write_index: u64,
    pub read_index: u64,
}

const_assert_eq!(size_of::<UsbHsRingHeader>(), 0x10);

// ---------------------------------------------------------------------------
// IPC input structs (crate-internal)
// ---------------------------------------------------------------------------

/// Input for CreateInterfaceAvailableEvent.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct CreateInterfaceAvailableEventIn {
    pub index: u8,
    pub _pad: u8,
    pub filter: UsbHsInterfaceFilter,
}

const_assert_eq!(size_of::<CreateInterfaceAvailableEventIn>(), 0x12);

/// Input for SubmitControlRequest (pre-2.0.0).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct SubmitControlRequestIn {
    pub b_request: u8,
    pub bm_request_type: u8,
    pub w_value: u16,
    pub w_index: u16,
    pub w_length: u16,
    pub timeout_in_ms: u32,
}

const_assert_eq!(size_of::<SubmitControlRequestIn>(), 0xC);

/// Input for CtrlXferAsync (2.0.0+).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct CtrlXferAsyncIn {
    pub bm_request_type: u8,
    pub b_request: u8,
    pub w_value: u16,
    pub w_index: u16,
    pub w_length: u16,
    pub buffer: u64,
}

const_assert_eq!(size_of::<CtrlXferAsyncIn>(), 0x10);

/// Input for OpenUsbEp.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct OpenUsbEpIn {
    pub max_urb_count: u16,
    pub _pad: u16,
    pub ep_type: u32,
    pub ep_number: u32,
    pub ep_direction: u32,
    pub max_xfer_size: u32,
}

const_assert_eq!(size_of::<OpenUsbEpIn>(), 0x14);

/// Input for endpoint SubmitRequest (pre-2.0.0).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct EpSubmitRequestIn {
    pub size: u32,
    pub timeout_in_ms: u32,
}

const_assert_eq!(size_of::<EpSubmitRequestIn>(), 0x8);

/// Input for endpoint PostBufferAsync (2.0.0+).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct EpPostBufferAsyncIn {
    pub size: u32,
    pub _pad: u32,
    pub buffer: u64,
    pub id: u64,
}

const_assert_eq!(size_of::<EpPostBufferAsyncIn>(), 0x18);

/// Input for endpoint BatchBufferAsync (2.0.0+).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct EpBatchBufferAsyncIn {
    pub urb_count: u32,
    pub unk1: u32,
    pub unk2: u32,
    pub _pad: u32,
    pub buffer: u64,
    pub id: u64,
}

const_assert_eq!(size_of::<EpBatchBufferAsyncIn>(), 0x20);

/// Input for endpoint CreateSmmuSpace (4.0.0+).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct EpCreateSmmuSpaceIn {
    pub size: u32,
    pub _pad: u32,
    pub buffer: u64,
}

const_assert_eq!(size_of::<EpCreateSmmuSpaceIn>(), 0x10);
