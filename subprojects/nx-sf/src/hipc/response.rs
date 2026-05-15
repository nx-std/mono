//! HIPC response parsing.

use core::mem::size_of;

use nx_svc::raw::Handle as RawHandle;
use zerocopy::FromBytes;

use super::wire::{
    Header, MIN_PREFIX_BUF_SIZE, RECV_LIST_WIRE_NONE, SpecialHeader, StaticDescriptor,
};

/// Parsed HIPC response from the server.
///
/// Constructed exclusively by [`parse_response`]; its existence is proof that
/// the wire bytes formed a well-formed response (all declared bytes fit, no
/// client→server descriptors, no receive list).
#[derive(Debug)]
pub struct Response<'a> {
    /// Data words (raw response data).
    pub data_words: &'a [u32],
    /// Copy handles received.
    pub copy_handles: &'a [RawHandle],
    /// Move handles received.
    pub move_handles: &'a [RawHandle],
}

/// Error returned by [`parse_response`].
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
}

/// Parses an HIPC response from the thread's IPC buffer.
///
/// Returns a typed error for any malformed wire shape — never panics on
/// untrusted input. See [`ResponseParseError`] for the failure cases.
///
/// Generic over the buffer size `N`; [`parse_prefix`] enforces at
/// monomorphization that `N >= MIN_PREFIX_BUF_SIZE`.
pub fn parse_response<const N: usize>(buf: &[u8; N]) -> Result<Response<'_>, ResponseParseError> {
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
    // subsequent slicing steps can rely on the fit without re-validating.
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

    // Size check above proves every slicing below fits.
    let (copy_handles, rest) = <[RawHandle]>::ref_from_prefix_with_elems(buf, num_copy_handles)
        .expect("internal: size check guarantees fit");
    let (move_handles, rest) = <[RawHandle]>::ref_from_prefix_with_elems(rest, num_move_handles)
        .expect("internal: size check guarantees fit");
    let (_statics, rest) = <[StaticDescriptor]>::ref_from_prefix_with_elems(rest, num_statics)
        .expect("internal: size check guarantees fit");
    let (data_words, _) = <[u32]>::ref_from_prefix_with_elems(rest, num_data_words)
        .expect("internal: size check guarantees fit");

    Ok(Response {
        data_words,
        copy_handles,
        move_handles,
    })
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

    // SAFETY: the const assertion above ensures `N >= MIN_PREFIX_BUF_SIZE`,
    // which is large enough to hold the header, so `ref_from_prefix` cannot fail.
    let (header, rest) =
        Header::ref_from_prefix(buf).expect("internal: TLR buffer fits HIPC header");
    if !header.has_special_header() {
        return (
            Prefix {
                header: *header,
                extras: None,
            },
            rest,
        );
    }

    // SAFETY: `rest` has at least IPC_BUFFER_SIZE - HEADER_SIZE (8) = 248 bytes remaining and
    // SpecialHeader is 4 bytes, so `ref_from_prefix` cannot fail.
    let (special_hdr, rest) =
        SpecialHeader::ref_from_prefix(rest).expect("internal: TLR buffer fits special header");

    let (pid, rest) = if special_hdr.send_pid() {
        // SAFETY: `rest` has at least 244 bytes remaining and ProcessId is 8 bytes,
        // so `ref_from_prefix` cannot fail.
        let (pid_ref, rest) =
            ProcessId::ref_from_prefix(rest).expect("internal: TLR buffer fits PID");
        (Some(*pid_ref), rest)
    } else {
        (None, rest)
    };

    let prefix = Prefix {
        header: *header,
        extras: Some(Extras {
            num_copy_handles: special_hdr.num_copy_handles(),
            num_move_handles: special_hdr.num_move_handles(),
            pid,
        }),
    };

    (prefix, rest)
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
    /// Decoded for completeness; `parse_response` currently discards it.
    #[allow(dead_code)]
    pid: Option<ProcessId>,
}
