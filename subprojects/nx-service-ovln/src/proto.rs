//! Overlay notification service protocol constants.

use nx_sf::ServiceName;

/// Service name for the overlay notification receiver service.
pub const SERVICE_NAME_RCV: ServiceName = ServiceName::new_truncate("ovln:rcv");

/// Service name for the overlay notification sender service.
pub const SERVICE_NAME_SND: ServiceName = ServiceName::new_truncate("ovln:snd");

// IReceiver manager commands

/// Opens a receiver sub-object.
pub const RCV_OPEN_RECEIVER: u32 = 0;

// IReceiver sub-object commands

/// Adds a source to the receiver.
pub const RECEIVER_ADD_SOURCE: u32 = 0;

/// Removes a source from the receiver.
pub const RECEIVER_REMOVE_SOURCE: u32 = 1;

/// Gets the receive event handle.
pub const RECEIVER_GET_RECEIVE_EVENT_HANDLE: u32 = 2;

/// Receives a message.
pub const RECEIVER_RECEIVE: u32 = 3;

/// Receives a message with a system tick.
pub const RECEIVER_RECEIVE_WITH_TICK: u32 = 4;

// ISender manager commands

/// Opens a sender sub-object.
pub const SND_OPEN_SENDER: u32 = 0;

// ISender sub-object commands

/// Sends a message.
pub const SENDER_SEND: u32 = 0;

/// Gets the count of unreceived messages.
pub const SENDER_GET_UNRECEIVED_MESSAGE_COUNT: u32 = 1;
