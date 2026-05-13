//! GPIO service (`gpio`) implementation.
//!
//! Provides access to the Switch's GPIO pads for reading/writing digital
//! signals and configuring interrupts.
//!
//! ## Usage
//!
//! 1. Connect to the GPIO manager via [`connect_cmif`].
//! 2. Open a pad session via [`GpioService::open_session`] or
//!    [`GpioService::open_session2`].
//! 3. Configure and read/write through the [`GpioPadSession`] wrapper.
//! 4. Sessions and the service are closed automatically on `Drop`.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::Session;

mod cmif;
mod proto;
mod types;

pub use self::{
    cmif::{
        BindInterruptError, DispatchInBoolError, DispatchInU32Error, DispatchInU32OutBoolError,
        DispatchNoIoError, DispatchOutBoolError, DispatchOutU32Error, OpenSession2Error,
        OpenSessionError,
    },
    proto::SERVICE_NAME,
    types::{GpioDirection, GpioInterruptMode, GpioInterruptStatus, GpioPadName, GpioValue},
};

/// GPIO manager service wrapper.
#[repr(transparent)]
pub struct GpioService(Session);

impl GpioService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> nx_svc::ipc::Handle {
        self.0.handle()
    }
}

/// CMIF protocol methods for the GPIO manager.
impl GpioService {
    /// Opens a pad session by pad name.
    #[inline]
    pub fn open_session(&self, name: GpioPadName) -> Result<GpioPadSession, OpenSessionError> {
        let service = cmif::open_session(self.0.handle(), name as u32)?;
        Ok(GpioPadSession(service))
    }

    /// Opens a pad session by device code (7.0.0+).
    #[inline]
    pub fn open_session2(
        &self,
        device_code: u32,
        access_mode: u32,
    ) -> Result<GpioPadSession, OpenSession2Error> {
        let service = cmif::open_session2(self.0.handle(), device_code, access_mode)?;
        Ok(GpioPadSession(service))
    }

    /// Checks if a wake event is active for the given pad name (pre-7.0.0).
    #[inline]
    pub fn is_wake_event_active(
        &self,
        name: GpioPadName,
    ) -> Result<bool, DispatchInU32OutBoolError> {
        cmif::is_wake_event_active(self.0.handle(), name as u32)
    }

    /// Checks if a wake event is active for the given device code (7.0.0+).
    #[inline]
    pub fn is_wake_event_active2(
        &self,
        device_code: u32,
    ) -> Result<bool, DispatchInU32OutBoolError> {
        cmif::is_wake_event_active2(self.0.handle(), device_code)
    }
}

/// GPIO pad session wrapper.
///
/// Represents an open session to a specific GPIO pad. Provides methods
/// for configuring direction, reading/writing values, and managing
/// interrupts.
pub struct GpioPadSession(Session);

impl GpioPadSession {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> nx_svc::ipc::Handle {
        self.0.handle()
    }
}

/// CMIF protocol methods for GPIO pad sessions.
impl GpioPadSession {
    /// Sets the pad direction.
    #[inline]
    pub fn set_direction(&self, direction: GpioDirection) -> Result<(), DispatchInU32Error> {
        cmif::pad_set_direction(self.0.handle(), direction as u32)
    }

    /// Gets the pad direction.
    #[inline]
    pub fn get_direction(&self) -> Result<GpioDirection, GetDirectionError> {
        let raw = cmif::pad_get_direction(self.0.handle()).map_err(GetDirectionError::Dispatch)?;
        match raw {
            0 => Ok(GpioDirection::Input),
            1 => Ok(GpioDirection::Output),
            _ => Err(GetDirectionError::InvalidValue(raw)),
        }
    }

    /// Sets the interrupt mode.
    #[inline]
    pub fn set_interrupt_mode(&self, mode: GpioInterruptMode) -> Result<(), DispatchInU32Error> {
        cmif::pad_set_interrupt_mode(self.0.handle(), mode as u32)
    }

    /// Gets the interrupt mode.
    #[inline]
    pub fn get_interrupt_mode(&self) -> Result<GpioInterruptMode, GetInterruptModeError> {
        let raw = cmif::pad_get_interrupt_mode(self.0.handle())
            .map_err(GetInterruptModeError::Dispatch)?;
        match raw {
            0 => Ok(GpioInterruptMode::LowLevel),
            1 => Ok(GpioInterruptMode::HighLevel),
            2 => Ok(GpioInterruptMode::RisingEdge),
            3 => Ok(GpioInterruptMode::FallingEdge),
            4 => Ok(GpioInterruptMode::AnyEdge),
            _ => Err(GetInterruptModeError::InvalidValue(raw)),
        }
    }

    /// Enables or disables the interrupt.
    #[inline]
    pub fn set_interrupt_enable(&self, enable: bool) -> Result<(), DispatchInBoolError> {
        cmif::pad_set_interrupt_enable(self.0.handle(), enable)
    }

    /// Gets whether the interrupt is enabled.
    #[inline]
    pub fn get_interrupt_enable(&self) -> Result<bool, DispatchOutBoolError> {
        cmif::pad_get_interrupt_enable(self.0.handle())
    }

    /// Gets the interrupt status (pre-17.0.0).
    #[inline]
    pub fn get_interrupt_status(&self) -> Result<GpioInterruptStatus, GetInterruptStatusError> {
        let raw = cmif::pad_get_interrupt_status(self.0.handle())
            .map_err(GetInterruptStatusError::Dispatch)?;
        match raw {
            0 => Ok(GpioInterruptStatus::Inactive),
            1 => Ok(GpioInterruptStatus::Active),
            _ => Err(GetInterruptStatusError::InvalidValue(raw)),
        }
    }

    /// Clears the interrupt status (pre-17.0.0).
    #[inline]
    pub fn clear_interrupt_status(&self) -> Result<(), DispatchNoIoError> {
        cmif::pad_clear_interrupt_status(self.0.handle())
    }

    /// Sets the pad output value.
    #[inline]
    pub fn set_value(&self, value: GpioValue) -> Result<(), DispatchInU32Error> {
        cmif::pad_set_value(self.0.handle(), value as u32)
    }

    /// Gets the pad input value.
    #[inline]
    pub fn get_value(&self) -> Result<GpioValue, GetValueError> {
        let raw = cmif::pad_get_value(self.0.handle()).map_err(GetValueError::Dispatch)?;
        match raw {
            0 => Ok(GpioValue::Low),
            1 => Ok(GpioValue::High),
            _ => Err(GetValueError::InvalidValue(raw)),
        }
    }

    /// Binds the interrupt and returns the raw event handle.
    #[inline]
    pub fn bind_interrupt(&self) -> Result<u32, BindInterruptError> {
        cmif::pad_bind_interrupt(&self.0)
    }

    /// Unbinds the interrupt.
    #[inline]
    pub fn unbind_interrupt(&self) -> Result<(), DispatchNoIoError> {
        cmif::pad_unbind_interrupt(self.0.handle())
    }

    /// Enables or disables debounce.
    #[inline]
    pub fn set_debounce_enabled(&self, enable: bool) -> Result<(), DispatchInBoolError> {
        cmif::pad_set_debounce_enabled(self.0.handle(), enable)
    }

    /// Gets whether debounce is enabled.
    #[inline]
    pub fn get_debounce_enabled(&self) -> Result<bool, DispatchOutBoolError> {
        cmif::pad_get_debounce_enabled(self.0.handle())
    }

    /// Sets the debounce time in milliseconds.
    #[inline]
    pub fn set_debounce_time(&self, ms: i32) -> Result<(), DispatchInU32Error> {
        cmif::pad_set_debounce_time(self.0.handle(), ms)
    }

    /// Gets the debounce time in milliseconds.
    #[inline]
    pub fn get_debounce_time(&self) -> Result<i32, DispatchOutU32Error> {
        cmif::pad_get_debounce_time(self.0.handle())
    }
}

/// Connects to the GPIO service using CMIF.
pub fn connect_cmif(sm: &SmService) -> Result<GpioService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::from_handle(handle, 0);

    Ok(GpioService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get gpio service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);

/// Error returned by [`GpioPadSession::get_direction`].
#[derive(Debug, thiserror::Error)]
pub enum GetDirectionError {
    #[error("dispatch failed")]
    Dispatch(#[source] DispatchOutU32Error),
    #[error("invalid direction value: {0}")]
    InvalidValue(u32),
}

/// Error returned by [`GpioPadSession::get_interrupt_mode`].
#[derive(Debug, thiserror::Error)]
pub enum GetInterruptModeError {
    #[error("dispatch failed")]
    Dispatch(#[source] DispatchOutU32Error),
    #[error("invalid interrupt mode value: {0}")]
    InvalidValue(u32),
}

/// Error returned by [`GpioPadSession::get_interrupt_status`].
#[derive(Debug, thiserror::Error)]
pub enum GetInterruptStatusError {
    #[error("dispatch failed")]
    Dispatch(#[source] DispatchOutU32Error),
    #[error("invalid interrupt status value: {0}")]
    InvalidValue(u32),
}

/// Error returned by [`GpioPadSession::get_value`].
#[derive(Debug, thiserror::Error)]
pub enum GetValueError {
    #[error("dispatch failed")]
    Dispatch(#[source] DispatchOutU32Error),
    #[error("invalid gpio value: {0}")]
    InvalidValue(u32),
}
