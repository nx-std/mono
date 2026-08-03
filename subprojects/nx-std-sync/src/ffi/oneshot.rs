use alloc::boxed::Box;
use core::ffi::c_void;

use crate::oneshot;

/// The type of the data that can be sent on the channel.
type DataType = *mut c_void;

/// Opaque struct for the sending-half of a one-shot channel.
#[repr(C)]
struct Sender(oneshot::Sender<DataType>);

/// Opaque struct for the receiving-half of a one-shot channel.
#[repr(C)]
struct Receiver(oneshot::Receiver<DataType>);

/// Creates a new one-shot channel.
///
/// The caller is responsible for freeing the sender and receiver with the appropriate `free` functions,
/// unless they are consumed by `send` or `recv`.
#[unsafe(no_mangle)]
unsafe extern "C" fn __nx_std_sync__oneshot_create(tx: *mut *mut Sender, rx: *mut *mut Receiver) {
    let (inner_tx, inner_rx) = oneshot::channel::<DataType>();
    unsafe { *tx = Box::into_raw(Box::new(Sender(inner_tx))) };
    unsafe { *rx = Box::into_raw(Box::new(Receiver(inner_rx))) };
}

/// Frees a `NxSyncOneshotSender`.
///
/// If `sender` is `NULL`, this function does nothing.
#[unsafe(no_mangle)]
unsafe extern "C" fn __nx_std_sync__oneshot_sender_free(tx: *mut Sender) {
    if tx.is_null() {
        return;
    }
    drop(unsafe { Box::from_raw(tx) });
}

/// Frees a `NxSyncOneshotReceiver`.
///
/// If `receiver` is `NULL`, this function does nothing.
#[unsafe(no_mangle)]
unsafe extern "C" fn __nx_std_sync__oneshot_receiver_free(rx: *mut Receiver) {
    if rx.is_null() {
        return;
    }
    drop(unsafe { Box::from_raw(rx) });
}

/// Sends a value on the channel, consuming the sender.
///
/// This function takes ownership of the `sender` and it must not be used again,
/// regardless of whether the send was successful.
///
/// \param sender The sender.
/// \param value The value to send.
/// \return 0 on success, -1 on failure (e.g., receiver was dropped).
#[unsafe(no_mangle)]
unsafe extern "C" fn __nx_std_sync__oneshot_send(tx: *mut Sender, value: DataType) -> i32 {
    if tx.is_null() {
        return -1;
    }

    match unsafe { Box::from_raw(tx) }.0.send(value) {
        Ok(()) => 0,
        // The receiver was dropped. `SendError` hands the value back so a Rust caller can
        // reclaim it, but this signature has no out-parameter to return it through, so the
        // C caller keeps whatever ownership it had before the call.
        Err(oneshot::SendError(_)) => -1,
    }
}

/// Receives a value from the channel, consuming the receiver.
///
/// This function takes ownership of the `receiver` and it must not be used again,
/// regardless of whether the receive was successful.
/// The received value is stored in `out_value`.
///
/// \param receiver The receiver.
/// \param out_value Pointer to write the received value to.
/// \return 0 on success, -1 on failure (e.g., sender was dropped).
#[unsafe(no_mangle)]
unsafe extern "C" fn __nx_std_sync__oneshot_recv(
    rx: *mut Receiver,
    out_value: *mut DataType,
) -> i32 {
    if rx.is_null() {
        return -1;
    }

    // Ownership is taken whatever the outcome, as this function's contract states: the box is
    // dropped when `receiver` leaves scope on every path below.
    let receiver = unsafe { Box::from_raw(rx) };

    // Checked before receiving, not after. Receiving first would take the channel's one and
    // only value and then have nowhere to put it, destroying it while reporting the same `-1`
    // that a dropped sender reports.
    if out_value.is_null() {
        return -1;
    }

    match receiver.0.recv() {
        Ok(value) => {
            unsafe { *out_value = value };
            0
        }
        // The sender was dropped without ever sending.
        Err(oneshot::RecvError) => -1,
    }
}
