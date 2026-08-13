//! The one body type a router's replies all carry.

use nx_sys_thread_tls::IPC_BUFFER_SIZE;

use crate::hipc::HipcPayload;

/// Capacity of an erased body.
///
/// Sized to the whole IPC buffer rather than to the space a body actually gets,
/// which is that minus the envelope and the protocol header. Nothing is gained
/// by computing the tighter bound: a body that overruns the real one is caught
/// by the reply writer's own size check either way, and this way the capacity
/// does not have to track a layout that varies with the protocol and the
/// descriptors attached.
const BODY_CAPACITY: usize = IPC_BUFFER_SIZE;

/// A reply body erased to bytes.
///
/// [`Router`](super::Router) needs every route to answer with the same
/// [`Response`](super::Response) type, but a handler returns whatever its
/// command produces. This is where those meet: the type erasure axum performs
/// with its own `Body`, which likewise exists so that `Response<Body>` is one
/// type whatever the handler returned.
///
/// # Why bytes rather than a boxed payload
///
/// axum's body is a stream of frames, so erasing it means boxing a trait object
/// and polling through a virtual call. A reply here is a fixed block of bytes
/// with a length known before it is written, so erasure can simply be that
/// block: [`new`](Self::new) runs the payload's encoder once into an inline
/// buffer. That trades one copy for the allocation and the indirect call the
/// boxed form would cost on every reply, on a path that is otherwise free of
/// both.
///
/// The buffer is inline rather than heap-allocated because the wire format
/// already bounds it: a message cannot exceed the thread's IPC buffer, so a
/// body that would not fit here could not have been sent anyway.
#[derive(Debug, Clone)]
pub struct Body {
    bytes: [u8; BODY_CAPACITY],
    /// Encoded length the source payload reported.
    ///
    /// Recorded even when it exceeds [`BODY_CAPACITY`], in which case `bytes`
    /// holds nothing and this value is what makes the reply writer reject the
    /// message. See [`new`](Self::new).
    len: usize,
}

impl Body {
    /// Erases `payload` by encoding it into the inline buffer.
    ///
    /// A payload too large to fit is **not** encoded, and the body remembers
    /// the length it asked for. That length is larger than the IPC buffer by
    /// construction, so the reply writer's size check rejects the message and
    /// the sender is told with a result code. Reporting it that way rather than
    /// failing here keeps this infallible, which is what lets
    /// [`IntoResponse`](super::IntoResponse) be infallible too, exactly as
    /// axum's is.
    pub fn new<P: HipcPayload>(payload: P) -> Self {
        let len = payload.encoded_len();
        let mut bytes = [0u8; BODY_CAPACITY];
        if len <= BODY_CAPACITY {
            payload.write_to(&mut bytes[..len]);
        }
        Self { bytes, len }
    }

    /// Erases a payload of no bytes at all.
    #[inline]
    pub fn empty() -> Self {
        Self {
            bytes: [0; BODY_CAPACITY],
            len: 0,
        }
    }

    /// Returns the encoded bytes, or `None` if the payload did not fit.
    #[inline]
    pub fn as_bytes(&self) -> Option<&[u8]> {
        self.bytes.get(..self.len)
    }
}

impl Default for Body {
    #[inline]
    fn default() -> Self {
        Self::empty()
    }
}

impl HipcPayload for Body {
    /// The length the erased payload reported, which is what the reply writer
    /// checks the message against.
    #[inline]
    fn encoded_len(&self) -> usize {
        self.len
    }

    /// Copies the encoded bytes out.
    ///
    /// Only reached once the reply writer has accepted `encoded_len`, so the
    /// oversize case that left `bytes` empty cannot arrive here.
    #[inline]
    fn write_to(&self, dst: &mut [u8]) {
        if let Some(bytes) = self.as_bytes() {
            dst[..bytes.len()].copy_from_slice(bytes);
        }
    }
}
