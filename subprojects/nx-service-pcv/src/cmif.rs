//! CMIF protocol operations for the PCV service.

use core::mem::size_of;

use nx_sf::service::{BufferAttr, DispatchError, Session};

use crate::{
    dispatch::{dispatch_in, dispatch_in_out},
    proto,
    types::{
        GetPossibleClockRatesIn, GetPossibleClockRatesOut, SetClockRateIn, SetVoltageEnabledIn,
    },
};

/// Sets the clock rate for a module (pre-8.0.0).
pub(crate) fn set_clock_rate(service: &Session, module: u32, hz: u32) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::SET_CLOCK_RATE,
        SetClockRateIn { module, hz },
    )
}

/// Gets the clock rate for a module (pre-8.0.0).
pub(crate) fn get_clock_rate(service: &Session, module: u32) -> Result<u32, DispatchError> {
    dispatch_in_out(service, proto::GET_CLOCK_RATE, module)
}

/// Gets the possible clock rates for a module (pre-8.0.0).
pub(crate) fn get_possible_clock_rates(
    service: &Session,
    module: u32,
    rates: &mut [u32],
) -> Result<GetPossibleClockRatesOut, DispatchError> {
    let input = GetPossibleClockRatesIn {
        module,
        max_count: rates.len() as i32,
    };

    // SAFETY: `input` lives on the stack until `.send()` returns.
    // `rates` is a caller-provided buffer valid for the lifetime of this call.
    let result = unsafe {
        service
            .dispatch(proto::GET_POSSIBLE_CLOCK_RATES)
            .in_raw(
                (&raw const input).cast::<u8>(),
                size_of::<GetPossibleClockRatesIn>(),
            )
            .out_size(size_of::<GetPossibleClockRatesOut>())
            .buffer(
                rates.as_mut_ptr().cast::<u8>(),
                core::mem::size_of_val(rates),
                BufferAttr::OUT.or(BufferAttr::HIPC_POINTER),
            )
            .send()?
    };

    // SAFETY: the response payload is at least `size_of::<GetPossibleClockRatesOut>()` bytes.
    Ok(unsafe {
        core::ptr::read_unaligned(result.data.as_ptr().cast::<GetPossibleClockRatesOut>())
    })
}

/// Sets the voltage-enabled state for a power domain (pre-8.0.0).
pub(crate) fn set_voltage_enabled(
    service: &Session,
    power_domain: u32,
    state: bool,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::SET_VOLTAGE_ENABLED,
        SetVoltageEnabledIn {
            state: u8::from(state),
            _pad: [0; 3],
            power_domain,
        },
    )
}

/// Gets the voltage-enabled state for a power domain (pre-8.0.0).
pub(crate) fn get_voltage_enabled(
    service: &Session,
    power_domain: u32,
) -> Result<bool, DispatchError> {
    let raw: u8 = dispatch_in_out(service, proto::GET_VOLTAGE_ENABLED, power_domain)?;
    Ok(raw & 1 != 0)
}
