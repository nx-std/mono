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
    Cmif(CmifVersion),
    /// TIPC, the protocol introduced in HOS 12.0.0: no magic, no domains, no
    /// pointer descriptors, command id in the message type.
    Tipc,
}

/// Which of CMIF's two header versions a message uses.
///
/// The header field is an integer on the wire, but only two values are
/// assigned, and the second exists solely to say a context token is present.
/// Modelling it as an integer beside a separate token would admit two states
/// the protocol has no meaning for - a token on a message that declared version
/// `0`, and a message declaring version `1` with no token to carry - so the
/// token lives inside the variant that has one.
///
/// This is the same closed-set treatment `http::Version` gets, and for the same
/// reason: a version is a member of a list the protocol fixes, not a number a
/// peer chooses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmifVersion {
    /// Header version `0`: no context token.
    Plain,
    /// Header version `1`: a context token rides along, and the reply echoes
    /// it.
    WithContext {
        /// The token to echo.
        token: u32,
    },
}
