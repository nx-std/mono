//! CMIF protocol operations for NFC/NFP/Mifare services.

pub(crate) mod mifare;
pub(crate) mod nfc;
pub(crate) mod nfp;

pub use self::{
    mifare::CreateInterfaceError as MifareCreateInterfaceError,
    nfc::CreateInterfaceError as NfcCreateInterfaceError,
    nfp::CreateInterfaceError as NfpCreateInterfaceError,
};
