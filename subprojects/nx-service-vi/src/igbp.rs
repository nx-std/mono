//! IGraphicBufferProducer (IGBP) wrapper over `Binder` + `Parcel`.
//!
//! This module mirrors libnx `display/buffer_producer.c` and provides typed
//! Rust entry points for the Android IGBP transact codes that the Switch's VI
//! stack exposes via `IHOSBinderDriverRelay`. Higher-level surfaces (native
//! window, framebuffer) build on this transport.
//!
//! Layering:
//! - `Binder` owns the session id, refcounting, and raw transact dispatch.
//! - `Parcel` owns the wire-format buffer (header + 4-byte aligned payload).
//! - The free functions below own only the per-command payload encoding,
//!   reply parsing, and Android-binder error mapping (see [`BinderError`]).
//!
//! All functions accept a `relay: &Session` (the `IHOSBinderDriverRelay`
//! session) and a `binder: &Binder` (the IGBP-side binder object). Both come
//! out of a `ViService` + `ViLayer` pair from the higher-level VI API. The
//! `BinderError` returned by each operation corresponds to libnx
//! `binderConvertErrorCode`.

use nx_sf::{
    error::{
        GENERIC_ERROR,
        ResultCode,
        ToResultCode,
    },
    service::Session,
};
use zerocopy::{
    FromBytes as _,
    IntoBytes as _,
};

use crate::{
    binder::{
        Binder,
        BinderError,
        TransactError,
    },
    parcel::Parcel,
};

/// Android interface descriptor written at the start of every IGBP parcel.
pub const IGBP_INTERFACE_DESCRIPTOR: &str = "android.gui.IGraphicBufferProducer";

/// IGBP transact codes (Android IGraphicBufferProducer ordinals plus the
/// Switch-specific `SET_PREALLOCATED_BUFFER`).
pub mod code {
    /// Binder framework's "first call" transact base.
    pub const BINDER_FIRST_CALL: u32 = 0x1;

    pub const REQUEST_BUFFER: u32 = BINDER_FIRST_CALL;
    pub const SET_BUFFER_COUNT: u32 = 0x2;
    pub const DEQUEUE_BUFFER: u32 = 0x3;
    pub const DETACH_BUFFER: u32 = 0x4;
    pub const DETACH_NEXT_BUFFER: u32 = 0x5;
    pub const ATTACH_BUFFER: u32 = 0x6;
    pub const QUEUE_BUFFER: u32 = 0x7;
    pub const CANCEL_BUFFER: u32 = 0x8;
    pub const QUERY: u32 = 0x9;
    pub const CONNECT: u32 = 0xA;
    pub const DISCONNECT: u32 = 0xB;
    pub const SET_SIDEBAND_STREAM: u32 = 0xC;
    pub const ALLOCATE_BUFFERS: u32 = 0xD;
    /// Switch-specific custom command.
    pub const SET_PREALLOCATED_BUFFER: u32 = 0xE;
}

/// `BqRect` — rectangle used in `BqBufferInput.crop`.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct BqRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// `BqFence` — single GPU fence (mirrors libnx `NvFence`).
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct BqFence {
    pub id: u32,
    pub value: u32,
}

/// `BqMultiFence` — bag of up to 4 GPU fences (mirrors libnx `NvMultiFence`).
///
/// Used by `dequeue_buffer`, `queue_buffer`, and `cancel_buffer` to propagate
/// GPU synchronization back to/from the IGBP server.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct BqMultiFence {
    pub num_fences: u32,
    pub fences: [BqFence; 4],
}

impl BqMultiFence {
    /// Builds a multi-fence holding a single [`BqFence`].
    #[inline]
    pub fn from_fence(fence: BqFence) -> Self {
        let mut fences = [BqFence::default(); 4];
        fences[0] = fence;
        Self {
            num_fences: 1,
            fences,
        }
    }
}

/// `BqBufferInput` — packed flat object passed to `queueBuffer`.
///
/// Wire-layout must match libnx `BqBufferInput` (`buffer_producer.h`).
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C, packed)]
pub struct BqBufferInput {
    /// Frame timestamp (ns), or 0 if `is_auto_timestamp` is set.
    pub timestamp: i64,
    /// Non-zero to instruct the server to derive the timestamp itself.
    pub is_auto_timestamp: i32,
    /// Source crop rectangle.
    pub crop: BqRect,
    /// `ViScalingMode` value.
    pub scaling_mode: i32,
    /// `NATIVE_WINDOW_TRANSFORM_*` bitfield.
    pub transform: u32,
    /// Sticky transform flags.
    pub sticky_transform: u32,
    /// Reserved / unknown.
    pub unk: u32,
    /// Swap interval (frames between flips).
    pub swap_interval: u32,
    /// Producer-side GPU fence(s).
    pub fence: BqMultiFence,
}

/// `BqBufferOutput` — wire-format struct returned by `queueBuffer` and
/// `connect`. Must match libnx `BqBufferOutput` layout byte-for-byte.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct BqBufferOutput {
    pub width: u32,
    pub height: u32,
    pub transform_hint: u32,
    pub num_pending_buffers: u32,
}

/// Performs the standard "write interface token, transact, propagate
/// `binderConvertErrorCode`" envelope shared by most IGBP calls.
///
/// `encode` is given a mutable parcel pre-loaded with the interface token to
/// append command-specific payload. `decode` is invoked once the transact has
/// succeeded (before the trailing binder rc has been consumed) so it can
/// extract typed reply data and convert the rc.
fn igbp_transact<E, D, R, Err>(
    binder: &Binder,
    relay: &Session,
    code: u32,
    encode: E,
    decode: D,
) -> Result<R, Err>
where
    E: FnOnce(&mut Parcel),
    D: FnOnce(&mut Parcel) -> Result<R, Err>,
    Err: From<TransactError>,
{
    let mut req = Parcel::new();
    req.write_interface_token(IGBP_INTERFACE_DESCRIPTOR);
    encode(&mut req);

    let mut reply = Parcel::new();
    binder.transact(relay, code, &req, &mut reply, 0)?;
    decode(&mut reply)
}

/// `bqRequestBuffer` — request the back-buffer for a given slot.
///
/// Returns `Ok(true)` when the server reports the slot is populated. Parsing
/// the embedded `GraphicBuffer` flat object is not yet implemented (matches
/// libnx, which also returns `LibnxError_BadInput` when a caller asks for the
/// parsed buffer).
pub fn request_buffer(
    binder: &Binder,
    relay: &Session,
    buffer_idx: i32,
) -> Result<bool, RequestBufferError> {
    igbp_transact(
        binder,
        relay,
        code::REQUEST_BUFFER,
        |req| {
            req.write_i32(buffer_idx);
        },
        |reply| {
            let non_null = reply.read_i32().ok_or(RequestBufferError::Malformed)?;
            if non_null != 0 {
                // libnx parses a GraphicBuffer flat object here but immediately
                // returns BadInput when the caller wants it. Mirror that: skip
                // past the flat object so we can read the trailing binder rc.
                reply
                    .read_flattened_object()
                    .ok_or(RequestBufferError::Malformed)?;
            }
            let rc = reply.read_i32().ok_or(RequestBufferError::Malformed)?;
            BinderError::from_code(rc).map_err(RequestBufferError::Binder)?;
            Ok(non_null != 0)
        },
    )
}

/// `bqSetBufferCount` — request the server allocate `count` buffers.
pub fn set_buffer_count(
    binder: &Binder,
    relay: &Session,
    count: i32,
) -> Result<(), SetBufferCountError> {
    igbp_transact(
        binder,
        relay,
        code::SET_BUFFER_COUNT,
        |req| {
            req.write_i32(count);
        },
        |reply| {
            let rc = reply.read_i32().ok_or(SetBufferCountError::Malformed)?;
            BinderError::from_code(rc).map_err(SetBufferCountError::Binder)
        },
    )
}

/// `bqDequeueBuffer` — request a free buffer slot from the server.
///
/// Returns the slot index and the producer-side fence (if any).
pub fn dequeue_buffer(
    binder: &Binder,
    relay: &Session,
    async_mode: bool,
    width: u32,
    height: u32,
    format: i32,
    usage: u32,
) -> Result<DequeueBufferOutput, DequeueBufferError> {
    igbp_transact(
        binder,
        relay,
        code::DEQUEUE_BUFFER,
        |req| {
            req.write_i32(async_mode as i32);
            req.write_u32(width);
            req.write_u32(height);
            req.write_i32(format);
            req.write_u32(usage);
        },
        |reply| {
            let slot = reply.read_i32().ok_or(DequeueBufferError::Malformed)?;
            let has_fence = reply.read_i32().ok_or(DequeueBufferError::Malformed)? != 0;

            let fence = if has_fence {
                let bytes = reply
                    .read_flattened_object()
                    .ok_or(DequeueBufferError::Malformed)?;
                let fence = BqMultiFence::read_from_bytes(bytes)
                    .map_err(|_| DequeueBufferError::Malformed)?;
                Some(fence)
            } else {
                None
            };

            let rc = reply.read_i32().ok_or(DequeueBufferError::Malformed)?;
            BinderError::from_code(rc).map_err(DequeueBufferError::Binder)?;
            Ok(DequeueBufferOutput { slot, fence })
        },
    )
}

/// `bqDetachBuffer` — detach a buffer slot from the queue.
pub fn detach_buffer(binder: &Binder, relay: &Session, slot: i32) -> Result<(), DetachBufferError> {
    igbp_transact(
        binder,
        relay,
        code::DETACH_BUFFER,
        |req| {
            req.write_i32(slot);
        },
        |reply| {
            let rc = reply.read_i32().ok_or(DetachBufferError::Malformed)?;
            BinderError::from_code(rc).map_err(DetachBufferError::Binder)
        },
    )
}

/// `bqQueueBuffer` — queue a filled buffer slot for presentation.
///
/// Returns the server-reported [`BqBufferOutput`] (window dimensions,
/// transform hint, pending-buffer count).
pub fn queue_buffer(
    binder: &Binder,
    relay: &Session,
    slot: i32,
    input: &BqBufferInput,
) -> Result<BqBufferOutput, QueueBufferError> {
    igbp_transact(
        binder,
        relay,
        code::QUEUE_BUFFER,
        |req| {
            req.write_i32(slot);
            req.write_flattened_object(input.as_bytes());
        },
        |reply| {
            let bytes = reply
                .read_data(core::mem::size_of::<BqBufferOutput>())
                .ok_or(QueueBufferError::Malformed)?;
            let output =
                BqBufferOutput::read_from_bytes(bytes).map_err(|_| QueueBufferError::Malformed)?;
            let rc = reply.read_i32().ok_or(QueueBufferError::Malformed)?;
            BinderError::from_code(rc).map_err(QueueBufferError::Binder)?;
            Ok(output)
        },
    )
}

/// `bqCancelBuffer` — return an unfilled buffer slot to the server.
pub fn cancel_buffer(
    binder: &Binder,
    relay: &Session,
    slot: i32,
    fence: &BqMultiFence,
) -> Result<(), CancelBufferError> {
    igbp_transact(
        binder,
        relay,
        code::CANCEL_BUFFER,
        |req| {
            req.write_i32(slot);
            req.write_flattened_object(fence.as_bytes());
        },
        |_reply| {
            // libnx: reply parcel has no content.
            Ok(())
        },
    )
}

/// `bqQuery` — query a single producer property (e.g. `NATIVE_WINDOW_FORMAT`).
pub fn query(binder: &Binder, relay: &Session, what: i32) -> Result<i32, QueryError> {
    igbp_transact(
        binder,
        relay,
        code::QUERY,
        |req| {
            req.write_i32(what);
        },
        |reply| {
            let value = reply.read_i32().ok_or(QueryError::Malformed)?;
            let rc = reply.read_i32().ok_or(QueryError::Malformed)?;
            BinderError::from_code(rc).map_err(QueryError::Binder)?;
            Ok(value)
        },
    )
}

/// `bqConnect` — connect to the producer with the given consumer API id.
///
/// Returns the server's initial [`BqBufferOutput`].
pub fn connect(
    binder: &Binder,
    relay: &Session,
    api: i32,
    producer_controlled_by_app: bool,
) -> Result<BqBufferOutput, ConnectError> {
    igbp_transact(
        binder,
        relay,
        code::CONNECT,
        |req| {
            // Hard-coded listener=NULL (mirrors libnx — listener objects are
            // not used in the Switch's IGBP).
            req.write_i32(0);
            req.write_i32(api);
            req.write_i32(producer_controlled_by_app as i32);
        },
        |reply| {
            let bytes = reply
                .read_data(core::mem::size_of::<BqBufferOutput>())
                .ok_or(ConnectError::Malformed)?;
            let output =
                BqBufferOutput::read_from_bytes(bytes).map_err(|_| ConnectError::Malformed)?;
            let rc = reply.read_i32().ok_or(ConnectError::Malformed)?;
            BinderError::from_code(rc).map_err(ConnectError::Binder)?;
            Ok(output)
        },
    )
}

/// `bqDisconnect` — disconnect from the producer.
pub fn disconnect(binder: &Binder, relay: &Session, api: i32) -> Result<(), DisconnectError> {
    igbp_transact(
        binder,
        relay,
        code::DISCONNECT,
        |req| {
            req.write_i32(api);
        },
        |reply| {
            let rc = reply.read_i32().ok_or(DisconnectError::Malformed)?;
            BinderError::from_code(rc).map_err(DisconnectError::Binder)
        },
    )
}

/// Maximum number of "ints" allowed in a `BqGraphicBufferInput`'s native
/// handle. Mirrors libnx's `num_ints > 0x80` rejection in
/// `bqSetPreallocatedBuffer`.
pub const MAX_GRAPHIC_BUFFER_NATIVE_INTS: usize = 0x80;

/// `bqSetPreallocatedBuffer` input.
///
/// Mirrors the inline `struct { u32 magic; u32 width; ... u32 numInts; u32 ints[N]; }`
/// that libnx serializes inside the parcel. The caller provides the buffer
/// metadata and the `native_handle_ints` payload extracted from the underlying
/// `NativeHandle`. `num_fds` is hard-coded to 0 (libnx does the same — only
/// "int" handles are supported on Switch).
#[derive(Debug)]
pub struct BqGraphicBufferInput<'a> {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: u32,
    pub usage: u32,
    /// Raw "ints" part of the underlying `NativeHandle` (no fds).
    pub native_handle_ints: &'a [u32],
}

/// `bqSetPreallocatedBuffer` — register a preallocated buffer at slot `slot`.
///
/// When `input` is `None`, the server is asked to clear the slot (matches
/// libnx's `hasInput=false` branch).
pub fn set_preallocated_buffer(
    binder: &Binder,
    relay: &Session,
    slot: i32,
    input: Option<&BqGraphicBufferInput<'_>>,
) -> Result<(), SetPreallocatedBufferError> {
    if let Some(buf) = input
        && buf.native_handle_ints.len() > MAX_GRAPHIC_BUFFER_NATIVE_INTS
    {
        return Err(SetPreallocatedBufferError::TooManyHandleInts);
    }

    igbp_transact(
        binder,
        relay,
        code::SET_PREALLOCATED_BUFFER,
        |req| {
            req.write_i32(slot);
            req.write_i32(input.is_some() as i32);
            if let Some(buf) = input {
                // Serialize the inline GraphicBuffer struct into a scratch
                // buffer, then push as a flat object.
                serialize_graphic_buffer_into(
                    &mut |bytes| {
                        req.write_flattened_object(bytes);
                    },
                    buf,
                );
            }
        },
        |_reply| {
            // libnx: reply parcel has no content.
            Ok(())
        },
    )
}

/// Magic value at the start of the serialized GraphicBuffer flat object.
/// `'GBFR'` in little-endian (matches libnx).
const GRAPHIC_BUFFER_MAGIC: u32 = 0x4742_4652;
/// libnx hard-codes `pid = 42` (the official software's `getpid()` mock).
const GRAPHIC_BUFFER_PID: u32 = 42;

/// Header bytes preceding the variable-length `ints[]` in the GraphicBuffer.
const GRAPHIC_BUFFER_HEADER_U32S: usize = 10; // magic..numInts inclusive

/// Encodes a `BqGraphicBufferInput` into bytes and hands them to `emit`.
fn serialize_graphic_buffer_into(emit: &mut dyn FnMut(&[u8]), input: &BqGraphicBufferInput<'_>) {
    let mut scratch = [0u32; GRAPHIC_BUFFER_HEADER_U32S + MAX_GRAPHIC_BUFFER_NATIVE_INTS];
    let total_words = GRAPHIC_BUFFER_HEADER_U32S + input.native_handle_ints.len();

    scratch[0] = GRAPHIC_BUFFER_MAGIC;
    scratch[1] = input.width;
    scratch[2] = input.height;
    scratch[3] = input.stride;
    scratch[4] = input.format;
    scratch[5] = input.usage;
    scratch[6] = GRAPHIC_BUFFER_PID;
    scratch[7] = 0; // refcount — ignored during marshalling
    scratch[8] = 0; // numFds
    scratch[9] = input.native_handle_ints.len() as u32;
    scratch
        [GRAPHIC_BUFFER_HEADER_U32S..GRAPHIC_BUFFER_HEADER_U32S + input.native_handle_ints.len()]
        .copy_from_slice(input.native_handle_ints);

    emit(scratch[..total_words].as_bytes());
}

/// Result of [`dequeue_buffer`].
#[derive(Debug, Clone, Copy)]
pub struct DequeueBufferOutput {
    /// Slot index returned by the server.
    pub slot: i32,
    /// Optional producer-side GPU fence(s).
    pub fence: Option<BqMultiFence>,
}

/// Error returned by [`request_buffer`].
#[derive(Debug, thiserror::Error)]
pub enum RequestBufferError {
    #[error("binder transact failed")]
    Transact(#[from] TransactError),
    #[error("malformed IGBP reply")]
    Malformed,
    #[error("binder rc indicates failure")]
    Binder(#[source] BinderError),
}

impl ToResultCode for RequestBufferError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::Binder(err) => err.to_rc(),
            // Rejected locally after a successful reply, so no server
            // named a code for it.
            Self::Transact(_) | Self::Malformed => GENERIC_ERROR,
        }
    }
}

/// Error returned by [`set_buffer_count`].
#[derive(Debug, thiserror::Error)]
pub enum SetBufferCountError {
    #[error("binder transact failed")]
    Transact(#[from] TransactError),
    #[error("malformed IGBP reply")]
    Malformed,
    #[error("binder rc indicates failure")]
    Binder(#[source] BinderError),
}

impl ToResultCode for SetBufferCountError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::Binder(err) => err.to_rc(),
            // Rejected locally after a successful reply, so no server
            // named a code for it.
            Self::Transact(_) | Self::Malformed => GENERIC_ERROR,
        }
    }
}

/// Error returned by [`dequeue_buffer`].
#[derive(Debug, thiserror::Error)]
pub enum DequeueBufferError {
    #[error("binder transact failed")]
    Transact(#[from] TransactError),
    #[error("malformed IGBP reply")]
    Malformed,
    #[error("binder rc indicates failure")]
    Binder(#[source] BinderError),
}

impl ToResultCode for DequeueBufferError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::Binder(err) => err.to_rc(),
            // Rejected locally after a successful reply, so no server
            // named a code for it.
            Self::Transact(_) | Self::Malformed => GENERIC_ERROR,
        }
    }
}

/// Error returned by [`detach_buffer`].
#[derive(Debug, thiserror::Error)]
pub enum DetachBufferError {
    #[error("binder transact failed")]
    Transact(#[from] TransactError),
    #[error("malformed IGBP reply")]
    Malformed,
    #[error("binder rc indicates failure")]
    Binder(#[source] BinderError),
}

impl ToResultCode for DetachBufferError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::Binder(err) => err.to_rc(),
            // Rejected locally after a successful reply, so no server
            // named a code for it.
            Self::Transact(_) | Self::Malformed => GENERIC_ERROR,
        }
    }
}

/// Error returned by [`queue_buffer`].
#[derive(Debug, thiserror::Error)]
pub enum QueueBufferError {
    #[error("binder transact failed")]
    Transact(#[from] TransactError),
    #[error("malformed IGBP reply")]
    Malformed,
    #[error("binder rc indicates failure")]
    Binder(#[source] BinderError),
}

impl ToResultCode for QueueBufferError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::Binder(err) => err.to_rc(),
            // Rejected locally after a successful reply, so no server
            // named a code for it.
            Self::Transact(_) | Self::Malformed => GENERIC_ERROR,
        }
    }
}

/// Error returned by [`cancel_buffer`].
#[derive(Debug, thiserror::Error)]
pub enum CancelBufferError {
    #[error("binder transact failed")]
    Transact(#[from] TransactError),
}

impl ToResultCode for CancelBufferError {
    fn to_rc(self) -> ResultCode {
        match self {
            // Rejected locally after a successful reply, so no server
            // named a code for it.
            Self::Transact(_) => GENERIC_ERROR,
        }
    }
}

/// Error returned by [`query`].
#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error("binder transact failed")]
    Transact(#[from] TransactError),
    #[error("malformed IGBP reply")]
    Malformed,
    #[error("binder rc indicates failure")]
    Binder(#[source] BinderError),
}

impl ToResultCode for QueryError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::Binder(err) => err.to_rc(),
            // Rejected locally after a successful reply, so no server
            // named a code for it.
            Self::Transact(_) | Self::Malformed => GENERIC_ERROR,
        }
    }
}

/// Error returned by [`connect`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    #[error("binder transact failed")]
    Transact(#[from] TransactError),
    #[error("malformed IGBP reply")]
    Malformed,
    #[error("binder rc indicates failure")]
    Binder(#[source] BinderError),
}

impl ToResultCode for ConnectError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::Binder(err) => err.to_rc(),
            // Rejected locally after a successful reply, so no server
            // named a code for it.
            Self::Transact(_) | Self::Malformed => GENERIC_ERROR,
        }
    }
}

/// Error returned by [`disconnect`].
#[derive(Debug, thiserror::Error)]
pub enum DisconnectError {
    #[error("binder transact failed")]
    Transact(#[from] TransactError),
    #[error("malformed IGBP reply")]
    Malformed,
    #[error("binder rc indicates failure")]
    Binder(#[source] BinderError),
}

impl ToResultCode for DisconnectError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::Binder(err) => err.to_rc(),
            // Rejected locally after a successful reply, so no server
            // named a code for it.
            Self::Transact(_) | Self::Malformed => GENERIC_ERROR,
        }
    }
}

/// Error returned by [`set_preallocated_buffer`].
#[derive(Debug, thiserror::Error)]
pub enum SetPreallocatedBufferError {
    #[error("binder transact failed")]
    Transact(#[from] TransactError),
    #[error(
        "native_handle_ints exceeds {} (libnx rejects with LibnxError_BadInput)",
        MAX_GRAPHIC_BUFFER_NATIVE_INTS
    )]
    TooManyHandleInts,
}

impl ToResultCode for SetPreallocatedBufferError {
    fn to_rc(self) -> ResultCode {
        match self {
            // Rejected locally after a successful reply, so no server
            // named a code for it.
            Self::Transact(_) | Self::TooManyHandleInts => GENERIC_ERROR,
        }
    }
}
