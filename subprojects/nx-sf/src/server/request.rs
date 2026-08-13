//! Inbound requests, in the shape a handler is written against.
//!
//! [`parse_request`] is the entry point: it takes a message the HIPC layer has
//! already framed, picks the protocol that framed it, and hands back the
//! protocol-neutral [`Request`] the rest of a server works with.

use nx_svc::raw::Handle as RawHandle;

use super::{
    command::CommandId,
    protocol::{
        CmifVersion,
        Protocol,
    },
};
use crate::{
    cmif,
    error::{
        GENERIC_ERROR,
        ResultCode,
        ToResultCode,
    },
    hipc::{
        self,
        BufferDescriptor,
        ProcessId,
        RecvList,
        StaticDescriptor,
    },
    tipc,
};

/// Highest message type CMIF assigns.
///
/// CMIF numbers its command types from zero upwards and stops at the
/// context-carrying control request.
// `CommandType` is `#[repr(u32)]` and its largest discriminant is 7, so the
// narrowing to the message type's own width round-trips.
const CMIF_MESSAGE_TYPE_MAX: u16 = cmif::CommandType::ControlWithContext as u16;

/// Header version value that says a context token rides along.
///
/// The only assigned value besides zero; see [`CmifVersion`].
const CMIF_VERSION_WITH_CONTEXT: u32 = 1;

/// Lowest message type TIPC assigns.
///
/// TIPC starts at its close type and runs upwards without bound, which is what
/// lets one server serve both protocols off the message type alone.
const TIPC_MESSAGE_TYPE_MIN: u16 = tipc::CommandType::Close as u16;

/// Decodes a framed HIPC message into a protocol-neutral request.
///
/// Takes the [`hipc::Request`] rather than the raw buffer because the two
/// halves answer different questions and both are needed: HIPC owns the
/// descriptors and handles the arguments live in, and the command protocol owns
/// the reading of the message type and the data words. Framing once and
/// layering on top of it is also what keeps the protocol decoders from reaching
/// past HIPC into bytes they do not own.
///
/// The protocol is chosen by the message type's range, which is how the two
/// protocols coexist on one transport: CMIF's command types occupy the low
/// values, and TIPC was assigned everything from its close type upwards
/// precisely so a server could tell them apart without being told in advance.
///
/// # Errors
///
/// Returns [`RequestParseError`] when the message type belongs to no protocol
/// this crate serves, or when the protocol decoder rejects the message.
pub fn parse_request<'a>(request: &hipc::Request<'a>) -> Result<Inbound<'a>, RequestParseError> {
    let raw_type = request.message_type.to_raw();

    if raw_type <= CMIF_MESSAGE_TYPE_MAX {
        let decoded = cmif::parse_request(request).map_err(RequestParseError::DecodeCmif)?;
        return match decoded {
            cmif::Request::Command(cmd) => Ok(Inbound::Command(from_cmif(request, cmd))),
            cmif::Request::Control(cmd) => Ok(Inbound::Control(from_cmif(request, cmd))),
            cmif::Request::Close => Ok(Inbound::Close),
        };
    }

    if raw_type >= TIPC_MESSAGE_TYPE_MIN {
        let decoded = tipc::parse_request(request).map_err(RequestParseError::DecodeTipc)?;
        return match decoded {
            tipc::Request::Command(cmd) => Ok(Inbound::Command(from_tipc(request, cmd))),
            tipc::Request::Close => Ok(Inbound::Close),
        };
    }

    Err(RequestParseError::UnassignedMessageType(raw_type))
}

/// Errors returned by [`parse_request`].
#[derive(Debug, thiserror::Error)]
pub enum RequestParseError {
    /// The message type falls in the gap between the range CMIF uses and the
    /// range TIPC uses.
    ///
    /// Occurs when a client writes a message type no protocol was ever
    /// assigned. Detected before either decoder runs, so nothing was read out
    /// of the message body.
    #[error("message type {0:#x} is assigned to no command protocol")]
    UnassignedMessageType(u16),
    /// The message type named CMIF, and the CMIF decoder rejected the message.
    ///
    /// Occurs for the command types this crate does not serve and for a data
    /// region that carries no well-formed CMIF header.
    #[error("Failed to decode the message as a CMIF request")]
    DecodeCmif(#[source] cmif::RequestParseError),
    /// The message type named TIPC, and the TIPC decoder rejected the message.
    ///
    /// Occurs when the TIPC decoder rejects a message type this function routed
    /// to it. The two ranges agree today, so this reports a disagreement
    /// between them rather than anything a client can send.
    #[error("Failed to decode the message as a TIPC request")]
    DecodeTipc(#[source] tipc::RequestParseError),
}

impl ToResultCode for RequestParseError {
    fn to_rc(self) -> ResultCode {
        // Every variant is a request rejected before any handler saw it, so no
        // service assigned it a code to forward. This is the same value both
        // protocol decoders report for their own halves of it.
        GENERIC_ERROR
    }
}

/// A decoded inbound message, classified by what answering it means.
///
/// The classification comes from the message type, so a caller that matches on
/// this cannot read a command id off a session close, nor route a control
/// request to the hosted interface.
#[derive(Debug)]
pub enum Inbound<'a> {
    /// An invocation of a method on the session's interface. Answered by the
    /// interface.
    Command(Request<'a>),
    /// A domain conversion, an object clone, or a pointer-buffer-size query.
    /// Answered by the framework rather than by the hosted interface, and
    /// produced only by CMIF: TIPC dropped domains and the control path with
    /// them.
    Control(Request<'a>),
    /// A session close. Carries no arguments, and is not replied to.
    Close,
}

/// A decoded request: a head and the argument bytes.
///
/// Both fields borrow the message buffer, so the value lives exactly as long as
/// the message it describes. That is what keeps a reply, written into the same
/// buffer, from being built out of bytes it has already overwritten.
///
/// # Descriptor targets are not validated
///
/// The descriptor slices in [`Parts`] report what the sender *declared*. A
/// [`BufferDescriptor`]'s address and size are numbers from the client, made
/// real only by the mapping the kernel established for the duration of the
/// request; a [`StaticDescriptor`]'s bytes were copied by the kernel into this
/// process's pointer buffer. Reading through either one means building a slice
/// from a raw address, so the head hands back the descriptors themselves: the
/// step that vouches for a target belongs to the layer that knows which mapping
/// is live.
#[derive(Debug)]
pub struct Request<'a> {
    head: Parts<'a>,
    body: &'a [u8],
}

impl<'a> Request<'a> {
    /// Assembles a request from a head and its argument bytes.
    #[inline]
    pub fn from_parts(head: Parts<'a>, body: &'a [u8]) -> Self {
        Self { head, body }
    }

    /// Returns the request head.
    #[inline]
    pub fn parts(&self) -> &Parts<'a> {
        &self.head
    }

    /// Returns the argument bytes.
    ///
    /// This is the raw remainder of the data-words region, not a sized argument
    /// tuple: it still holds the word padding HIPC added and, for a CMIF
    /// command declaring out-pointers, the out-pointer-size table at its tail.
    /// How much of it is arguments follows from the command's own signature,
    /// which a wire-format decoder does not know.
    #[inline]
    pub fn body(&self) -> &'a [u8] {
        self.body
    }

    /// Splits the request into its head and its argument bytes.
    #[inline]
    pub fn into_parts(self) -> (Parts<'a>, &'a [u8]) {
        (self.head, self.body)
    }

    /// Returns the command id, the field a router keys on.
    #[inline]
    pub fn command(&self) -> CommandId {
        self.head.command
    }

    /// Returns the protocol the request was decoded from.
    #[inline]
    pub fn protocol(&self) -> Protocol {
        self.head.protocol
    }
}

/// The head of a request: everything but the argument bytes.
///
/// Every field is either `Copy` or a borrow of the message buffer, so a handler
/// can reach any of it without consuming the request.
#[derive(Debug)]
pub struct Parts<'a> {
    /// Method to invoke on the target interface.
    pub command: CommandId,
    /// Protocol the request arrived over, and the per-message state its reply
    /// must echo.
    pub protocol: Protocol,
    /// Sender's process id, present when the client asked for it to be sent.
    ///
    /// The kernel fills this slot itself, so it identifies the sender rather
    /// than repeating a claim the sender made.
    pub process_id: Option<ProcessId>,
    /// Handles the kernel duplicated into this process.
    pub copy_handles: &'a [RawHandle],
    /// Handles whose ownership transferred to this process.
    pub move_handles: &'a [RawHandle],
    /// Type-X send statics: pointer data the kernel copied in.
    pub send_statics: &'a [StaticDescriptor],
    /// Type-A send buffers: client memory mapped read-only.
    pub send_buffers: &'a [BufferDescriptor],
    /// Type-B receive buffers: client memory mapped read-write, for output.
    pub recv_buffers: &'a [BufferDescriptor],
    /// Type-W exchange buffers: client memory mapped read-write, bidirectional.
    pub exch_buffers: &'a [BufferDescriptor],
    /// Type-C receive list: where returned pointer data is to be written.
    pub recv_list: RecvList<'a>,
}

/// Merges a decoded CMIF command with the framing it arrived in.
fn from_cmif<'a>(frame: &hipc::Request<'a>, command: cmif::Command<'a>) -> Request<'a> {
    // The header's version field comes from the client, so it is read for the
    // one distinction the protocol assigns and nothing else: any value other
    // than the context-carrying one is answered as an unversioned message,
    // which is what a peer that wrote a number nobody assigned can still read.
    let version = if command.version == CMIF_VERSION_WITH_CONTEXT {
        CmifVersion::WithContext {
            token: command.token,
        }
    } else {
        CmifVersion::Plain
    };
    Request::from_parts(
        frame_head(
            frame,
            CommandId::new(command.command_id),
            Protocol::Cmif(version),
        ),
        command.payload,
    )
}

/// Merges a decoded TIPC command with the framing it arrived in.
fn from_tipc<'a>(frame: &hipc::Request<'a>, command: tipc::Command<'a>) -> Request<'a> {
    Request::from_parts(
        frame_head(frame, CommandId::new(command.command_id), Protocol::Tipc),
        command.payload,
    )
}

/// Builds the head shared by both protocols from the HIPC framing.
///
/// The descriptors and handles are the framing's, verbatim: which of them a
/// command uses is a property of its signature, and neither this function nor
/// the protocol decoder below it knows that signature.
fn frame_head<'a>(frame: &hipc::Request<'a>, command: CommandId, protocol: Protocol) -> Parts<'a> {
    Parts {
        command,
        protocol,
        process_id: frame.process_id,
        copy_handles: frame.copy_handles,
        move_handles: frame.move_handles,
        send_statics: frame.send_statics,
        send_buffers: frame.send_buffers,
        recv_buffers: frame.recv_buffers,
        exch_buffers: frame.exch_buffers,
        recv_list: frame.recv_list,
    }
}
