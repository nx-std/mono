//! Producer-side native window (libnx `NWindow`).
//!
//! Mirrors `display/native_window.c` in libnx: owns a binder session to an
//! IGBP server, tracks the slot table, dimensions, crop, transform, swap
//! interval, and current dequeued slot. Buffer configuration
//! ([`NativeWindow::configure_buffer`]) and the GPU-fence-aware
//! dequeue/queue/cancel loop are left for a follow-up that depends on the
//! NV graphic-buffer port (see `nx-service-nv`).

use core::cell::UnsafeCell;

use nx_service_vi::{
    Binder,
    BinderObjectId,
    binder::{
        InitSessionError,
        TransactError,
    },
    igbp::{
        self,
        BqBufferInput,
        BqBufferOutput,
        BqMultiFence,
        BqRect,
        ConnectError as IgbpConnectError,
        DequeueBufferError,
        DequeueBufferOutput,
        DisconnectError as IgbpDisconnectError,
        QueueBufferError,
        RequestBufferError,
        SetPreallocatedBufferError,
    },
};
use nx_sf::service::Session;
use nx_svc::raw::Handle as RawHandle;

/// Maximum number of slots a single IGBP queue can hold (Android-side limit).
pub const NATIVE_WINDOW_MAX_SLOTS: usize = 64;

/// Producer API id passed in `bqConnect` / `bqDisconnect`.
pub type NativeWindowApi = i32;
pub const NATIVE_WINDOW_API_EGL: NativeWindowApi = 1;
pub const NATIVE_WINDOW_API_CPU: NativeWindowApi = 2;
pub const NATIVE_WINDOW_API_MEDIA: NativeWindowApi = 3;
pub const NATIVE_WINDOW_API_CAMERA: NativeWindowApi = 4;

/// `HAL_TRANSFORM_*` bitfield accepted by [`NativeWindow::set_transform`].
#[derive(Debug, Clone, Copy, Default)]
pub struct Transform(pub u32);

impl Transform {
    pub const FLIP_H: u32 = 0x01;
    pub const FLIP_V: u32 = 0x02;
    pub const ROT_90: u32 = 0x04;
    /// Mask of bits libnx accepts on input.
    pub const VALID_MASK: u32 = Self::FLIP_H | Self::FLIP_V | Self::ROT_90;
}

/// Producer-side native window.
///
/// Holds the binder session, slot bitmasks, current dequeued slot, and the
/// user-configured dimensions/crop/transform/swap-interval. All mutation is
/// internally serialized via [`UnsafeCell`] + a single-threaded contract; the
/// type is `!Sync` by default.
pub struct NativeWindow {
    // Mutable state hidden behind UnsafeCell — methods that mutate accept
    // `&self` to mirror libnx's API ergonomics (functions take `NWindow*`,
    // not const-pointer).
    inner: UnsafeCell<Inner>,
}

struct Inner {
    /// IGBP-side binder.
    bq: Binder,
    /// IHOSBinderDriverRelay session pointer. Borrowed for the window's
    /// lifetime; the caller owns it (typically the runtime's `ViService`).
    relay: *const Session,
    /// VSync / fence event handle returned by `binderGetNativeHandle(0x0f)`.
    /// libnx wraps this in an `Event`; we keep the raw handle here and let
    /// callers wrap it with their preferred event abstraction.
    event: RawHandle,
    /// True after a successful `bqConnect`.
    is_connected: bool,
    /// Bitmask of slots that have been registered via
    /// `bqSetPreallocatedBuffer`. Bit `n` set means slot `n` is configured.
    slots_configured: u64,
    /// Bitmask of slots that have been promoted via `bqRequestBuffer` since
    /// the last `_nwindowDisconnect`.
    slots_requested: u64,
    /// Currently dequeued slot, or -1 when no slot is checked out.
    cur_slot: i32,
    /// Configured frame width (0 = inherit from buffer).
    width: u32,
    /// Configured frame height (0 = inherit from buffer).
    height: u32,
    /// Configured pixel format (`!0` = inherit from buffer).
    format: u32,
    /// Configured usage flags (0 = inherit from buffer).
    usage: u32,
    /// Last-known default dimensions reported by the server.
    default_width: u32,
    default_height: u32,
    /// `consumer_running_behind` hint from `BqBufferOutput.num_pending_buffers`.
    consumer_running_behind: bool,
    /// Active crop rectangle.
    crop: BqRect,
    /// `HAL_TRANSFORM_*` bitfield.
    transform: u32,
    /// Sticky transform applied on top of `transform` by the server.
    sticky_transform: u32,
    /// `ViScalingMode` value.
    scaling_mode: i32,
    /// Frames-between-flips.
    swap_interval: u32,
    /// Whether the producer is controlled by the app (vs. the system).
    producer_controlled_by_app: bool,
}

impl NativeWindow {
    /// Builds a native window connected to the IGBP at `binder_id`.
    ///
    /// `relay` is the `IHOSBinderDriverRelay` session retained by the caller
    /// (typically the runtime's `ViService`). It must outlive the
    /// `NativeWindow`.
    pub fn create(
        relay: &Session,
        binder_id: BinderObjectId,
        producer_controlled_by_app: bool,
    ) -> Result<Self, NativeWindowError> {
        let mut bq = Binder::create(binder_id);
        bq.init_session(relay).map_err(NativeWindowError::Binder)?;

        // Fence/vsync event — libnx fetches handle 0x0f.
        let event = bq
            .get_native_handle(relay, 0x0f)
            .map_err(|e| NativeWindowError::NativeHandle(NativeHandleErrorKind::from_cmif(e)))?;

        let mut inner = Inner {
            bq,
            relay: relay as *const Session,
            event,
            is_connected: false,
            slots_configured: 0,
            slots_requested: 0,
            cur_slot: -1,
            width: 0,
            height: 0,
            format: !0,
            usage: 0,
            default_width: 0,
            default_height: 0,
            consumer_running_behind: false,
            crop: BqRect::default(),
            transform: 0,
            sticky_transform: 0,
            scaling_mode: 0,
            swap_interval: 1,
            producer_controlled_by_app,
        };

        // Connect using NATIVE_WINDOW_API_CPU (matches libnx default).
        let connect_result = igbp::connect(
            &inner.bq,
            // SAFETY: `relay` outlives this call.
            unsafe { &*inner.relay },
            NATIVE_WINDOW_API_CPU,
            inner.producer_controlled_by_app,
        );
        match connect_result {
            Ok(output) => {
                inner.is_connected = true;
                inner.apply_output(&output);
            }
            Err(err) => {
                // Best-effort cleanup; we ignore close errors because there's
                // nothing actionable left to do at this point.
                let relay_ref = unsafe { &*inner.relay };
                inner.bq.close(relay_ref);
                return Err(NativeWindowError::Connect(err));
            }
        }

        Ok(Self {
            inner: UnsafeCell::new(inner),
        })
    }

    /// Returns the VSync / fence event handle.
    #[inline]
    pub fn event_handle(&self) -> RawHandle {
        // SAFETY: read-only access to a field of `Inner`.
        unsafe { (*self.inner.get()).event }
    }

    /// Returns the currently effective frame dimensions.
    ///
    /// When `set_dimensions` has not been called, falls back to the server's
    /// `default_*` values (updated by every successful `queueBuffer`/connect).
    pub fn dimensions(&self) -> (u32, u32) {
        // SAFETY: read-only.
        let inner = unsafe { &*self.inner.get() };
        let w = if inner.width != 0 {
            inner.width
        } else {
            inner.default_width
        };
        let h = if inner.height != 0 {
            inner.height
        } else {
            inner.default_height
        };
        (w, h)
    }

    /// Sets the frame dimensions (libnx `nwindowSetDimensions`).
    ///
    /// Returns [`NativeWindowError::AlreadyInitialized`] if dimensions have
    /// already been frozen by a prior `configure_buffer` call.
    pub fn set_dimensions(&self, width: u32, height: u32) -> Result<(), NativeWindowError> {
        // SAFETY: methods on NativeWindow are single-threaded by contract.
        let inner = unsafe { &mut *self.inner.get() };
        if (inner.width != 0 || inner.height != 0) && inner.slots_configured != 0 {
            return Err(NativeWindowError::AlreadyInitialized);
        }
        inner.width = width;
        inner.height = height;
        inner.crop = BqRect::default();
        Ok(())
    }

    /// Sets the source crop rectangle (libnx `nwindowSetCrop`).
    ///
    /// Coordinates are clamped to the active dimensions.
    pub fn set_crop(
        &self,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    ) -> Result<(), NativeWindowError> {
        if right < left || bottom < top {
            return Err(NativeWindowError::BadInput);
        }
        let (width, height) = self.dimensions();
        // SAFETY: single-threaded mutation.
        let inner = unsafe { &mut *self.inner.get() };
        inner.crop.left = left.max(0);
        inner.crop.top = top.max(0);
        inner.crop.right = right.min(width as i32);
        inner.crop.bottom = bottom.min(height as i32);
        Ok(())
    }

    /// Sets the active transform bits (libnx `nwindowSetTransform`).
    pub fn set_transform(&self, transform: u32) -> Result<(), NativeWindowError> {
        if transform & !Transform::VALID_MASK != 0 {
            return Err(NativeWindowError::BadInput);
        }
        // SAFETY: single-threaded mutation.
        let inner = unsafe { &mut *self.inner.get() };
        inner.transform = transform;
        Ok(())
    }

    /// Sets the swap interval in frames (libnx `nwindowSetSwapInterval`).
    pub fn set_swap_interval(&self, swap_interval: u32) -> Result<(), NativeWindowError> {
        // SAFETY: single-threaded mutation.
        let inner = unsafe { &mut *self.inner.get() };
        inner.swap_interval = swap_interval;
        Ok(())
    }

    /// Returns whether the consumer is reporting that pending buffers are
    /// piling up. Updated on every successful `queue_buffer` / `connect`.
    pub fn consumer_running_behind(&self) -> bool {
        // SAFETY: read-only.
        unsafe { (*self.inner.get()).consumer_running_behind }
    }

    /// Registers a preallocated buffer at `slot` (libnx
    /// `nwindowConfigureBuffer`'s `bqSetPreallocatedBuffer` half).
    ///
    /// This entry point is for callers that already have a serialized
    /// `BqGraphicBufferInput` (the NV-side bits live in a future revision of
    /// `nx-service-nv`). `width`, `height`, `format`, and `usage` are derived
    /// from the buffer on the first call to mirror libnx.
    pub fn configure_buffer(
        &self,
        slot: i32,
        buffer: &igbp::BqGraphicBufferInput<'_>,
    ) -> Result<(), NativeWindowError> {
        if !(0..NATIVE_WINDOW_MAX_SLOTS as i32).contains(&slot) {
            return Err(NativeWindowError::BadInput);
        }
        // SAFETY: single-threaded mutation.
        let inner = unsafe { &mut *self.inner.get() };
        let slot_mask = 1u64 << slot;
        if inner.slots_configured & slot_mask != 0 {
            return Err(NativeWindowError::AlreadyInitialized);
        }

        // libnx re-connects if the prior session was disconnected.
        if !inner.is_connected {
            inner.reconnect()?;
        }

        if inner.width == 0 {
            inner.width = buffer.width;
        }
        if inner.height == 0 {
            inner.height = buffer.height;
        }
        if inner.format == !0 {
            inner.format = buffer.format;
        }
        if inner.usage == 0 {
            inner.usage = buffer.usage;
        }

        // The IGBP wrapper expects the caller's stride/format/usage fields,
        // but libnx overrides them from the window's accumulated state. Build
        // a shadow input that mirrors that behavior.
        let bq_input = igbp::BqGraphicBufferInput {
            width: inner.width,
            height: inner.height,
            stride: buffer.stride,
            format: inner.format,
            usage: inner.usage,
            native_handle_ints: buffer.native_handle_ints,
        };

        let relay = unsafe { &*inner.relay };
        igbp::set_preallocated_buffer(&inner.bq, relay, slot, Some(&bq_input))
            .map_err(NativeWindowError::SetPreallocatedBuffer)?;

        inner.slots_configured |= slot_mask;
        Ok(())
    }

    /// Releases all dequeued buffers and disconnects from the producer
    /// (libnx `nwindowReleaseBuffers`).
    pub fn release_buffers(&self) -> Result<(), NativeWindowError> {
        // SAFETY: single-threaded.
        let inner = unsafe { &mut *self.inner.get() };

        if inner.cur_slot >= 0 {
            let slot = inner.cur_slot;
            inner.cancel_buffer_internal(slot, None)?;
        }

        if inner.is_connected && inner.slots_configured != 0 {
            inner.disconnect_internal()?;
        }

        Ok(())
    }

    /// Dequeues a free buffer slot (libnx `nwindowDequeueBuffer`).
    ///
    /// Returns the slot index plus the optional GPU fence. Caller is
    /// responsible for waiting on the fence before writing to the buffer.
    pub fn dequeue_buffer(&self) -> Result<DequeueBufferOutput, NativeWindowError> {
        // SAFETY: single-threaded.
        let inner = unsafe { &mut *self.inner.get() };
        if inner.slots_configured == 0 || inner.cur_slot >= 0 {
            return Err(NativeWindowError::BadDequeue);
        }
        let relay = unsafe { &*inner.relay };

        let out = igbp::dequeue_buffer(
            &inner.bq,
            relay,
            false,
            inner.width,
            inner.height,
            inner.format as i32,
            inner.usage,
        )
        .map_err(NativeWindowError::DequeueBuffer)?;

        let slot_mask = 1u64 << out.slot;
        if inner.slots_requested & slot_mask == 0 {
            // Mirror libnx: the very first dequeue of a slot must be promoted
            // through `requestBuffer`. On request-buffer failure, cancel.
            if let Err(req_err) = igbp::request_buffer(&inner.bq, relay, out.slot) {
                let _ = igbp::cancel_buffer(
                    &inner.bq,
                    relay,
                    out.slot,
                    out.fence.as_ref().unwrap_or(&BqMultiFence::default()),
                );
                return Err(NativeWindowError::RequestBuffer(req_err));
            }
            inner.slots_requested |= slot_mask;
        }

        inner.cur_slot = out.slot;
        Ok(out)
    }

    /// Queues a previously dequeued buffer for presentation (libnx
    /// `nwindowQueueBuffer`).
    pub fn queue_buffer(
        &self,
        slot: i32,
        fence: Option<&BqMultiFence>,
    ) -> Result<(), NativeWindowError> {
        if !(0..NATIVE_WINDOW_MAX_SLOTS as i32).contains(&slot) {
            return Err(NativeWindowError::BadInput);
        }
        // SAFETY: single-threaded.
        let inner = unsafe { &mut *self.inner.get() };
        if slot != inner.cur_slot {
            return Err(NativeWindowError::BadQueue);
        }

        let bq_input = BqBufferInput {
            timestamp: 0,
            is_auto_timestamp: 0,
            crop: inner.crop,
            scaling_mode: inner.scaling_mode,
            transform: inner.transform,
            sticky_transform: inner.sticky_transform,
            unk: 0,
            swap_interval: inner.swap_interval,
            fence: fence.copied().unwrap_or_default(),
        };

        let relay = unsafe { &*inner.relay };
        let output = igbp::queue_buffer(&inner.bq, relay, slot, &bq_input)
            .map_err(NativeWindowError::QueueBuffer)?;
        inner.cur_slot = -1;
        inner.apply_output(&output);
        Ok(())
    }

    /// Cancels a previously dequeued buffer (libnx `nwindowCancelBuffer`).
    pub fn cancel_buffer(
        &self,
        slot: i32,
        fence: Option<&BqMultiFence>,
    ) -> Result<(), NativeWindowError> {
        if !(0..NATIVE_WINDOW_MAX_SLOTS as i32).contains(&slot) {
            return Err(NativeWindowError::BadInput);
        }
        // SAFETY: single-threaded.
        let inner = unsafe { &mut *self.inner.get() };
        if slot != inner.cur_slot {
            return Err(NativeWindowError::BadQueue);
        }
        inner.cancel_buffer_internal(slot, fence)
    }

    /// Drops the native window, disconnecting and closing the binder.
    ///
    /// Consumes `self` so that the borrow checker enforces no further use.
    pub fn close(self) {
        // SAFETY: single-threaded.
        let inner = unsafe { &mut *self.inner.get() };
        if inner.is_connected {
            let _ = inner.disconnect_internal();
        }
        let relay = unsafe { &*inner.relay };
        inner.bq.close(relay);
    }
}

impl Inner {
    fn apply_output(&mut self, out: &BqBufferOutput) {
        self.default_width = out.width;
        self.default_height = out.height;
        self.consumer_running_behind = out.num_pending_buffers > 1;
    }

    fn reconnect(&mut self) -> Result<(), NativeWindowError> {
        let relay = unsafe { &*self.relay };
        let output = igbp::connect(
            &self.bq,
            relay,
            NATIVE_WINDOW_API_CPU,
            self.producer_controlled_by_app,
        )
        .map_err(NativeWindowError::Connect)?;
        self.is_connected = true;
        self.apply_output(&output);
        Ok(())
    }

    fn disconnect_internal(&mut self) -> Result<(), NativeWindowError> {
        let relay = unsafe { &*self.relay };
        igbp::disconnect(&self.bq, relay, NATIVE_WINDOW_API_CPU)
            .map_err(NativeWindowError::Disconnect)?;
        self.is_connected = false;
        self.slots_configured = 0;
        self.slots_requested = 0;
        self.cur_slot = -1;
        self.width = 0;
        self.height = 0;
        self.format = 0;
        self.usage = 0;
        Ok(())
    }

    fn cancel_buffer_internal(
        &mut self,
        slot: i32,
        fence: Option<&BqMultiFence>,
    ) -> Result<(), NativeWindowError> {
        let empty = BqMultiFence::default();
        let fence_ref = fence.unwrap_or(&empty);
        let relay = unsafe { &*self.relay };
        igbp::cancel_buffer(&self.bq, relay, slot, fence_ref)
            .map_err(NativeWindowError::CancelBuffer)?;
        self.cur_slot = -1;
        Ok(())
    }
}

// SAFETY: NativeWindow holds raw pointers + Binder state and is designed for
// single-threaded use. Send is fine (handles are integers, not pointers into
// thread-local state); Sync is intentionally NOT implemented.
unsafe impl Send for NativeWindow {}

/// Internal kind for native-handle-acquisition failure.
#[derive(Debug, thiserror::Error)]
pub enum NativeHandleErrorKind {
    /// Underlying CMIF/transact-parcel call failed.
    #[error("get_native_handle failed")]
    Cmif,
}

impl NativeHandleErrorKind {
    fn from_cmif(_: nx_service_vi::GetNativeHandleError) -> Self {
        Self::Cmif
    }
}

/// Error returned by the [`NativeWindow`] surface.
#[derive(Debug, thiserror::Error)]
pub enum NativeWindowError {
    #[error("invalid input")]
    BadInput,
    #[error("native window already initialized at this slot/dimension")]
    AlreadyInitialized,
    #[error("invalid dequeue state")]
    BadDequeue,
    #[error("invalid queue/cancel state (slot mismatch)")]
    BadQueue,
    #[error("binder init_session failed")]
    Binder(#[source] InitSessionError),
    #[error("binder transact failed")]
    Transact(#[from] TransactError),
    #[error("get_native_handle failed")]
    NativeHandle(#[source] NativeHandleErrorKind),
    #[error("IGBP connect failed")]
    Connect(#[source] IgbpConnectError),
    #[error("IGBP disconnect failed")]
    Disconnect(#[source] IgbpDisconnectError),
    #[error("IGBP set_preallocated_buffer failed")]
    SetPreallocatedBuffer(#[source] SetPreallocatedBufferError),
    #[error("IGBP dequeue_buffer failed")]
    DequeueBuffer(#[source] DequeueBufferError),
    #[error("IGBP queue_buffer failed")]
    QueueBuffer(#[source] QueueBufferError),
    #[error("IGBP cancel_buffer failed")]
    CancelBuffer(#[source] nx_service_vi::igbp::CancelBufferError),
    #[error("IGBP request_buffer failed")]
    RequestBuffer(#[source] RequestBufferError),
}
