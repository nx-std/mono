//! HIPC response parsing.

use core::mem::size_of;

use nx_svc::raw::Handle as RawHandle;

use super::wire::{
    Header, MIN_PREFIX_BUF_SIZE, RECV_LIST_WIRE_NONE, SpecialHeader, StaticDescriptor,
};
use crate::cursor::{Cursor, ResponsePayload};

/// Parses a full HIPC response into the envelope plus a typed payload.
///
/// Generic over `P: ResponsePayload`: callers pick the payload shape via
/// turbofish — `&T` for a zerocopy struct or `()` for no in-band
/// payload.
///
/// Returns a typed error for any malformed wire shape — never panics on
/// untrusted input. See [`ResponseParseError`] for the failure cases.
///
/// Generic over the buffer size `N`; [`parse_prefix`] enforces at
/// monomorphization that `N >= MIN_PREFIX_BUF_SIZE`.
pub fn parse_response<'a, const N: usize, P>(
    buf: &'a [u8; N],
) -> Result<Response<'a, P>, ResponseParseError>
where
    P: ResponsePayload<'a>,
{
    let envelope = parse_response_envelope(buf)?;
    let cursor = Cursor::new(envelope.data_words);
    let (payload, _) = P::read(cursor).ok_or(ResponseParseError::TruncatedPayload)?;

    Ok(Response {
        payload,
        copy_handles: envelope.copy_handles,
        move_handles: envelope.move_handles,
    })
}

/// Parses the HIPC envelope and exposes the raw data-words region.
///
/// Used by CMIF and TIPC, which build their own cursor over
/// `data_words` to walk their protocol-specific headers before
/// delegating to a [`ResponsePayload`] for the user payload.
pub fn parse_response_envelope<const N: usize>(
    buf: &[u8; N],
) -> Result<Envelope<'_>, ResponseParseError> {
    let (prefix, buf) = parse_prefix(buf);
    let header = &prefix.header;

    if header.num_send_buffers() != 0
        || header.num_recv_buffers() != 0
        || header.num_exch_buffers() != 0
    {
        return Err(ResponseParseError::UnexpectedBufferDescriptor);
    }
    if header.recv_static_mode() != RECV_LIST_WIRE_NONE {
        return Err(ResponseParseError::UnexpectedRecvList);
    }

    let (num_copy_handles, num_move_handles) = match &prefix.extras {
        Some(extras) => (
            extras.num_copy_handles as usize,
            extras.num_move_handles as usize,
        ),
        None => (0, 0),
    };
    let num_statics = header.num_send_statics() as usize;
    let num_data_words = header.num_data_words() as usize;

    // Bound-check the declared payload against the buffer once so the
    // subsequent cursor reads can rely on the fit without re-validating.
    let declared = num_copy_handles * size_of::<RawHandle>()
        + num_move_handles * size_of::<RawHandle>()
        + num_statics * size_of::<StaticDescriptor>()
        + num_data_words * size_of::<u32>();
    if declared > buf.len() {
        return Err(ResponseParseError::DeclaredSizeExceedsBuffer {
            declared,
            capacity: buf.len(),
        });
    }

    // Size check above proves every cursor read below fits.
    let cursor = Cursor::new(buf);
    let (copy_handles, cursor) = cursor
        .read_slice::<RawHandle>(num_copy_handles)
        .expect("internal: size check guarantees fit");
    let (move_handles, cursor) = cursor
        .read_slice::<RawHandle>(num_move_handles)
        .expect("internal: size check guarantees fit");
    let (_statics, cursor) = cursor
        .read_slice::<StaticDescriptor>(num_statics)
        .expect("internal: size check guarantees fit");
    let (data_words, _) = cursor
        .read_bytes(num_data_words * size_of::<u32>())
        .expect("internal: size check guarantees fit");

    Ok(Envelope {
        data_words,
        copy_handles,
        move_handles,
    })
}

/// Error returned by [`parse_response`] and [`parse_response_envelope`].
#[derive(Debug, thiserror::Error)]
pub enum ResponseParseError {
    /// The header's declared descriptor counts imply a message longer than
    /// [`IPC_BUFFER_SIZE`], so the response cannot be decoded without reading
    /// past the end of the TLR buffer.
    #[error("HIPC response declares {declared} bytes but only {capacity} remain in buffer")]
    DeclaredSizeExceedsBuffer {
        /// Total descriptor-region bytes implied by the header's counts.
        declared: usize,
        /// Bytes available after the decoded prefix.
        capacity: usize,
    },
    /// Response carries A/B/W buffer descriptors. These are client→server only;
    /// a server reply must not carry them.
    #[error("HIPC response carries client→server buffer descriptors")]
    UnexpectedBufferDescriptor,
    /// Response declares a Type-C receive list. The receive list is a
    /// request-side construct used by clients to pre-allocate buffers for
    /// server pointer descriptors; it has no meaning in a reply.
    #[error("HIPC response declares a receive-list mode")]
    UnexpectedRecvList,
    /// Data-words region too small to hold the caller-requested payload.
    #[error("HIPC response too small for payload")]
    TruncatedPayload,
}

/// Parsed HIPC response with a typed payload.
///
/// Returned by [`parse_response`]. The payload type is whatever the
/// caller selected via the `P` type parameter.
#[derive(Debug)]
pub struct Response<'a, P> {
    /// In-band payload, parsed from the data-words region.
    pub payload: P,
    /// Copy handles received.
    pub copy_handles: &'a [RawHandle],
    /// Move handles received.
    pub move_handles: &'a [RawHandle],
}

/// Parsed HIPC envelope with the raw data-words region exposed.
///
/// Returned by [`parse_response_envelope`]. CMIF and TIPC consume this
/// shape to walk their own protocol headers before exposing the user
/// payload.
#[derive(Debug)]
pub struct Envelope<'a> {
    /// Raw data-words region, as bytes. The kernel reserves
    /// `num_data_words * 4` bytes here for the protocol payload.
    pub data_words: &'a [u8],
    /// Copy handles received.
    pub copy_handles: &'a [RawHandle],
    /// Move handles received.
    pub move_handles: &'a [RawHandle],
}

/// Decodes the wire-level prefix (header + optional special header + optional PID)
/// from a fixed-size IPC buffer slice, returning the parsed [`Prefix`] and the
/// remainder of the buffer after the prefix bytes.
///
/// # Wire layout
///
/// Three valid prefix shapes, selected by two bits in the wire bytes:
///
/// ```text
/// offset:   0                  8              12             20
///           ┌──────────────────┬──────────────┬──────────────┐
/// shape A:  │      Header      │ ⋯ payload ⋯
///           │     (8 bytes)    │
///           └──────────────────┘
///           has_special_header = 0
///
///           ┌──────────────────┬──────────────┬──────────────┐
/// shape B:  │      Header      │ SpecialHdr   │ ⋯ payload ⋯
///           │     (8 bytes)    │  (4 bytes)   │
///           └──────────────────┴──────────────┘
///           has_special_header = 1, send_pid = 0
///
///           ┌──────────────────┬──────────────┬──────────────┐
/// shape C:  │      Header      │ SpecialHdr   │  ProcessId   │ ⋯ payload ⋯
///           │     (8 bytes)    │  (4 bytes)   │  (8 bytes)   │
///           └──────────────────┴──────────────┴──────────────┘
///           has_special_header = 1, send_pid = 1
/// ```
///
/// Infallible: the caller supplies a buffer at least [`MIN_PREFIX_BUF_SIZE`]
/// bytes long (the worst-case prefix), enforced at monomorphization via a
/// `const` assertion on `N`.
fn parse_prefix<const N: usize>(buf: &[u8; N]) -> (Prefix, &[u8]) {
    // Compile-time check: the buffer must fit the worst-case prefix
    const {
        assert!(
            N >= MIN_PREFIX_BUF_SIZE,
            "parse_prefix buffer must be at least MIN_PREFIX_BUF_SIZE bytes",
        );
    }

    // The const assertion above ensures `N >= MIN_PREFIX_BUF_SIZE`, so the
    // cursor reads below cannot fail.
    let cursor = Cursor::new(buf);
    let (header, cursor) = cursor
        .read::<Header>()
        .expect("internal: TLR buffer fits HIPC header");
    if !header.has_special_header() {
        return (
            Prefix {
                header: *header,
                extras: None,
            },
            cursor.remaining(),
        );
    }

    let (special_hdr, cursor) = cursor
        .read::<SpecialHeader>()
        .expect("internal: TLR buffer fits special header");

    let (pid, cursor) = if special_hdr.send_pid() {
        let (pid_ref, cursor) = cursor
            .read::<ProcessId>()
            .expect("internal: TLR buffer fits PID");
        (Some(*pid_ref), cursor)
    } else {
        (None, cursor)
    };

    let prefix = Prefix {
        header: *header,
        extras: Some(Extras {
            num_copy_handles: special_hdr.num_copy_handles(),
            num_move_handles: special_hdr.num_move_handles(),
            pid,
        }),
    };

    (prefix, cursor.remaining())
}

/// Parsed wire-level prefix shared by requests and responses.
///
/// The three valid prefix shapes on the wire — header only / header + special
/// header / header + special header + PID — collapse into a single value with
/// nested `Option`s, so a consumer cannot read the discriminant bits
/// independently from the bytes they describe.
#[derive(Debug, Clone, Copy)]
struct Prefix {
    /// Wire-level message header (always present).
    header: Header,
    /// Optional special header + PID payload. `None` ⇔ the header's
    /// `has_special_header` bit is clear.
    extras: Option<Extras>,
}

/// Sender's process ID as decoded from the HIPC special header payload.
///
/// The HIPC wire layout stores the PID as a native-endian `u64` immediately
/// after the special header, so the type derives the zerocopy byte-conversion
/// traits and is decoded in place by `parse_prefix`.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(transparent)]
struct ProcessId(u64);

/// Decoded contents of the special header section.
#[derive(Debug, Clone, Copy)]
struct Extras {
    /// Number of copy handles (≤ 15, bound by the `B4` bitfield width).
    num_copy_handles: u8,
    /// Number of move handles (≤ 15, bound by the `B4` bitfield width).
    num_move_handles: u8,
    /// Sender's process ID, if the `send_pid` bit was set on the wire.
    /// Decoded for completeness; the parser currently discards it.
    #[allow(dead_code)]
    pid: Option<ProcessId>,
}
