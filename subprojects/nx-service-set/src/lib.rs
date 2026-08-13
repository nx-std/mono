//! Settings services implementation.
//!
//! What a console was configured with, and what it was configured by: the language and region it
//! was set up in, the firmware it runs, and the settings items the system reads its own knobs
//! out of.
//!
//! ## Interfaces
//!
//! The settings live behind two interfaces, and a session to one answers nothing about the
//! other:
//!
//! - **`set`** - the settings a console is set up with: language, region, nickname. Connected
//!   via [`connect_cmif`].
//! - **`set:sys`** - the system's own settings: firmware version, theme, settings items.
//!   Connected via [`connect_sys_cmif`].
//!
//! A third interface, `set:cal`, holds the calibration a console was manufactured with. Nothing
//! here reaches it yet.
//!
//! ## Protocol
//!
//! Every command is CMIF, except the firmware version, which is also reachable over TIPC.
//! The `_tipc` connect functions choose how the *Service Manager* is asked for the interface,
//! which `[12.0.0]` and Atmosphère changed; they do not change what the session's own commands
//! are sent as.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

mod dispatch;
mod set;
mod set_sys;

pub use self::{
    set::{
        AvailableLanguageCodes,
        ConnectCmifError,
        ConnectTipcError,
        DeviceNickname,
        GetAvailableLanguageCodesError,
        GetLanguageCodeError,
        GetRegionCodeError,
        InvalidLanguageCode,
        Language,
        LanguageCode,
        LanguageCodeParseError,
        RegionCode,
        SetService,
        UnknownLanguage,
        UnknownRegionCode,
        connect_cmif,
        connect_tipc,
    },
    set_sys::{
        ColorSetId,
        ConnectSysCmifError,
        ConnectSysTipcError,
        FirmwareVersion,
        GetColorSetIdError,
        GetFirmwareVersionCmifError,
        GetFirmwareVersionTipcError,
        GetSettingsItemValueU64Error,
        InvalidSettingsText,
        SetSysService,
        SettingsItemKey,
        SettingsName,
        UnknownColorSetId,
        connect_sys_cmif,
        connect_sys_tipc,
    },
};
