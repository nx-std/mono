//! Which command protocol a session speaks.

/// Command protocol an inbound message was decoded from.
///
/// The counterpart of `http::Version`, and it earns the comparison: like
/// HTTP/1.1 and HTTP/2, CMIF and TIPC express the same conversation over
/// incompatible bytes, so the protocol has to ride along with a decoded message
/// for the reply to be encodable.
///
/// It is not named `Version` because CMIF's own in-band header has a `version`
/// field, and the two answer different questions.
///
/// The CMIF variant carries the per-message state a reply has to echo. TIPC
/// carries nothing: its reply is a result code and a payload, fully determined
/// by the [`Response`](super::Response) itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    /// CMIF, the original protocol: magic-headed, domain-capable, and the only
    /// one of the two with control requests.
    Cmif {
        /// Header version the request declared: `1` when a context token rides
        /// along, `0` otherwise.
        version: u32,
        /// Context token to echo back in the reply.
        token: u32,
    },
    /// TIPC, the protocol introduced in HOS 12.0.0: no magic, no domains, no
    /// pointer descriptors, command id in the message type.
    Tipc,
}
