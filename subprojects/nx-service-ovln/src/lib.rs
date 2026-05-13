//! Overlay notification service (`ovln:rcv`, `ovln:snd`) implementation.
//!
//! Provides inter-process overlay notification messaging on the Switch.
//!
//! ## Usage
//!
//! ### Receiving
//! 1. Connect to the receiver service via [`connect_rcv_cmif`].
//! 2. Open a receiver sub-object via [`OvlnRcvService::open_receiver`].
//! 3. Add sources, wait on the event, and receive messages.
//!
//! ### Sending
//! 1. Connect to the sender service via [`connect_snd_cmif`].
//! 2. Open a sender sub-object via [`OvlnSndService::open_sender`].
//! 3. Send messages through the sender.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::Session;

mod cmif;
mod proto;
mod types;

pub use self::{
    cmif::{
        DispatchInError, DispatchOutError, GetReceiveEventHandleError, OpenReceiverError,
        OpenSenderError,
    },
    proto::{SERVICE_NAME_RCV, SERVICE_NAME_SND},
    types::{
        OvlnEnqueuePosition, OvlnOverflowOption, OvlnQueueAttribute, OvlnRawMessage,
        OvlnSendOption, OvlnSourceName, ReceiveWithTickOut,
    },
};

/// Overlay notification receiver manager service wrapper.
#[repr(transparent)]
pub struct OvlnRcvService(Session);

impl OvlnRcvService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> nx_svc::ipc::Handle {
        self.0.handle()
    }

    /// Opens a receiver sub-object.
    #[inline]
    pub fn open_receiver(&self) -> Result<OvlnReceiver, OpenReceiverError> {
        let service = cmif::rcv_open_receiver(self.0.handle())?;
        Ok(OvlnReceiver(service))
    }
}

/// Overlay notification sender manager service wrapper.
#[repr(transparent)]
pub struct OvlnSndService(Session);

impl OvlnSndService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> nx_svc::ipc::Handle {
        self.0.handle()
    }

    /// Opens a sender sub-object with the given source name and queue attributes.
    #[inline]
    pub fn open_sender(
        &self,
        name: &OvlnSourceName,
        attribute: &OvlnQueueAttribute,
    ) -> Result<OvlnSender, OpenSenderError> {
        let service = cmif::snd_open_sender(self.0.handle(), name, attribute)?;
        Ok(OvlnSender(service))
    }
}

/// Overlay notification receiver sub-object.
pub struct OvlnReceiver(Session);

impl OvlnReceiver {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> nx_svc::ipc::Handle {
        self.0.handle()
    }

    /// Adds a source to this receiver.
    #[inline]
    pub fn add_source(&self, name: &OvlnSourceName) -> Result<(), DispatchInError> {
        cmif::receiver_add_source(self.0.handle(), name)
    }

    /// Removes a source from this receiver.
    #[inline]
    pub fn remove_source(&self, name: &OvlnSourceName) -> Result<(), DispatchInError> {
        cmif::receiver_remove_source(self.0.handle(), name)
    }

    /// Gets the receive event handle (copy handle, autoclear=true).
    #[inline]
    pub fn get_receive_event_handle(&self) -> Result<u32, GetReceiveEventHandleError> {
        cmif::receiver_get_receive_event_handle(self.0.handle())
    }

    /// Receives a message.
    #[inline]
    pub fn receive(&self) -> Result<OvlnRawMessage, DispatchOutError> {
        cmif::receiver_receive(self.0.handle())
    }

    /// Receives a message with the associated system tick.
    #[inline]
    pub fn receive_with_tick(&self) -> Result<ReceiveWithTickOut, DispatchOutError> {
        cmif::receiver_receive_with_tick(self.0.handle())
    }
}

/// Overlay notification sender sub-object.
pub struct OvlnSender(Session);

impl OvlnSender {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> nx_svc::ipc::Handle {
        self.0.handle()
    }

    /// Sends a message with the given options.
    #[inline]
    pub fn send(
        &self,
        option: &OvlnSendOption,
        message: &OvlnRawMessage,
    ) -> Result<(), DispatchInError> {
        cmif::sender_send(self.0.handle(), option, message)
    }

    /// Gets the count of unreceived messages.
    #[inline]
    pub fn get_unreceived_message_count(&self) -> Result<u32, DispatchOutError> {
        cmif::sender_get_unreceived_message_count(self.0.handle())
    }
}

/// Connects to the overlay notification receiver service (`ovln:rcv`) using CMIF.
pub fn connect_rcv_cmif(sm: &SmService) -> Result<OvlnRcvService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME_RCV)
        .map_err(ConnectCmifError)?;

    let service = Session::from_handle(handle, 0);

    Ok(OvlnRcvService(service))
}

/// Connects to the overlay notification sender service (`ovln:snd`) using CMIF.
pub fn connect_snd_cmif(sm: &SmService) -> Result<OvlnSndService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME_SND)
        .map_err(ConnectCmifError)?;

    let service = Session::from_handle(handle, 0);

    Ok(OvlnSndService(service))
}

/// Error returned by [`connect_rcv_cmif`] and [`connect_snd_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get ovln service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
