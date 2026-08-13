//! Hosting a port: the loop that turns arriving messages into handler calls.
//!
//! # Why there is no per-session task
//!
//! `hyper` gives a connection its own task and blocks it on that socket, so a
//! server is a loop of accepts that spawns one per client. Nothing here can be
//! shaped that way: the kernel's reply-and-receive call waits on the port and
//! *every* open session at once and reports which of them woke it, and there is
//! no per-session blocking call to build a task around. One [`Server`] is
//! therefore one thread serving every client, and its wait set is the whole
//! state of the exchange.
//!
//! # Where the reply lives
//!
//! A reply is not sent by a call of its own. It is written into the thread's
//! IPC buffer and handed to the *next* reply-and-receive, which sends it and
//! then blocks. So the loop always runs one step behind itself: it answers the
//! previous request and waits for the next in the same syscall, and a round
//! that has nothing to answer simply passes no reply target.

use alloc::vec::Vec;

use nx_svc::{
    ipc::{
        Handle as SessionHandle,
        PortHandle,
        ReplyAndReceiveError,
        accept_session,
        reply_and_receive,
    },
    raw::Handle as RawHandle,
};

use super::{
    protocol::{
        CmifVersion,
        Protocol,
    },
    request::{
        Inbound,
        parse_request,
    },
    response::Response,
    service::Service,
};
use crate::{
    error::{
        LibnxError,
        ToResultCode as _,
        libnx_error,
    },
    hipc::{
        self,
        HipcPayload,
    },
    service::OwnedSessionHandle,
};

/// Position of the port in the wait set.
///
/// The port is kept first so a session's position in `sessions` is its wait-set
/// index minus one, which is the only arithmetic relating the two.
const PORT_INDEX: usize = 0;

/// A hosted port and the sessions open on it, serving one interface.
///
/// Built around a [`Service`], which answers the commands, and a
/// [`PortHandle`], which supplies the clients. [`run`](Self::run) drives both
/// until the loop is asked to stop.
pub struct Server<S: Service> {
    service: S,
    port: PortHandle,
    max_sessions: usize,
    sessions: Vec<OwnedSessionHandle>,
    /// Scratch for the wait set handed to each syscall.
    ///
    /// Rebuilt from `sessions` every round rather than maintained alongside it,
    /// so `sessions` stays the single record of which sessions are open. It is
    /// a field only to keep its capacity across rounds; nothing reads it
    /// between calls.
    wait_set: Vec<RawHandle>,
}

impl<S: Service> Server<S> {
    /// Hosts `service` on `port`, accepting up to 32 concurrent sessions.
    ///
    /// Change the cap with [`with_max_sessions`](Self::with_max_sessions).
    #[inline]
    pub fn new(port: PortHandle, service: S) -> Self {
        const DEFAULT_MAX_SESSIONS: usize = 32;

        Self {
            service,
            port,
            max_sessions: DEFAULT_MAX_SESSIONS,
            sessions: Vec::new(),
            wait_set: Vec::new(),
        }
    }

    /// Sets how many sessions may be open at once.
    ///
    /// A client that connects while the server is full has its session accepted
    /// and immediately closed, which it observes as the server hanging up. The
    /// alternative - leaving it pending on the port - would leave the port
    /// signalled and spin this loop.
    #[inline]
    pub fn with_max_sessions(mut self, max_sessions: usize) -> Self {
        self.max_sessions = max_sessions;
        self
    }

    /// Serves the port until the loop is asked to stop.
    ///
    /// Returns `Ok(())` when another thread cancels the wait or the process is
    /// terminating, which are the two ways a server is told to shut down. Every
    /// other kernel failure of the wait ends the loop as an error.
    ///
    /// # Every failure a client can see is a result code
    ///
    /// This protocol has no channel for a failure other than the result code in
    /// the reply, so a request that goes wrong is still answered. That covers a
    /// message that would not decode, a control request this crate does not
    /// serve, and a handler's own reply being too large or carrying pointer
    /// data the session's protocol cannot express: each becomes a code on that
    /// session and the loop moves on. Neither a client writing a bad message
    /// nor a handler building a bad reply can take the server down.
    ///
    /// What ends the loop is what no client is waiting on: the kernel refusing
    /// the wait, the port closing, or an accept failing.
    ///
    /// # Panics
    ///
    /// Panics if this thread's IPC buffer is already borrowed. The buffer is a
    /// per-thread singleton and the loop holds it for its whole run, so nothing
    /// else on this thread may hold it.
    ///
    /// # Pointer data is not yet servable
    ///
    /// A reply carrying returned pointer data would put a send static in the
    /// IPC buffer, and the memory it names has to stay live and unwritten until
    /// the *next* round's syscall has sent it. Nothing here keeps such a loan,
    /// and nothing needs to yet: [`StaticDescriptor`] has no public
    /// constructor, so no [`Response`] a service builds can carry one. Adding
    /// that constructor means giving this loop somewhere to hold the loan
    /// across the round, not just somewhere to put the descriptor.
    ///
    /// [`StaticDescriptor`]: crate::hipc::StaticDescriptor
    ///
    /// # Errors
    ///
    /// Returns [`ServeError`] when the kernel refuses the wait, when the port
    /// closes, or when even a bare failure reply cannot be delivered - the last
    /// meaning the server can no longer answer anyone at all.
    pub fn run(mut self) -> Result<(), ServeError> {
        let mut buf = nx_sys_thread_tls::ipc_buffer();
        let mut reply_target: Option<SessionHandle> = None;

        loop {
            self.refresh_wait_set();

            // SAFETY: no borrow of the IPC buffer is live across this call -
            // `answer` takes the token by unique reference and returned before
            // this line, so every borrow it handed out is already invalidated.
            // The reply the buffer holds carries no descriptor whose target
            // must outlive the call: `Response` accepts pointer data only as a
            // `StaticDescriptor`, which has no public constructor, so no reply
            // reaching here has one. See `run`'s note on pointer data for what
            // must be added before that stops being true.
            let signalled = match unsafe { reply_and_receive(&self.wait_set, reply_target, None) } {
                Ok(index) => index,
                Err(ReplyAndReceiveError::SessionClosed { index }) => {
                    self.drop_session(index)?;
                    reply_target = None;
                    continue;
                }
                Err(ReplyAndReceiveError::Cancelled)
                | Err(ReplyAndReceiveError::TerminationRequested) => return Ok(()),
                Err(err) => return Err(ServeError::Wait(err)),
            };

            // The reply that was pending has now been sent, whatever happens
            // to the message that arrived in its place.
            reply_target = None;

            if signalled == PORT_INDEX {
                self.accept()?;
                continue;
            }

            let session_index = signalled - 1;
            match self.answer(&mut buf)? {
                Answer::Replied => {
                    reply_target = self
                        .sessions
                        .get(session_index)
                        .map(|session| session.as_borrowed().to_handle());
                }
                Answer::Closed => self.drop_session(signalled)?,
            }
        }
    }

    /// Rebuilds the wait set as the port followed by every open session.
    ///
    /// Clearing and refilling reuses the capacity, so this allocates only while
    /// the session count is still growing past its high-water mark.
    fn refresh_wait_set(&mut self) {
        self.wait_set.clear();
        self.wait_set.push(self.port.to_raw());
        self.wait_set.extend(
            self.sessions
                .iter()
                .map(|session| session.as_borrowed().to_handle().to_raw()),
        );
    }

    /// Takes the session pending on the port.
    ///
    /// A session accepted while the table is full is closed again immediately,
    /// by being dropped here rather than stored.
    fn accept(&mut self) -> Result<(), ServeError> {
        let session = accept_session(self.port).map_err(ServeError::Accept)?;
        // SAFETY: the kernel just opened this session for this process and
        // nothing else holds it, so this value is its only owner and closing
        // it on drop is correct.
        let session = OwnedSessionHandle::from_handle_unchecked(session);

        if self.sessions.len() < self.max_sessions {
            self.sessions.push(session);
        }
        // Otherwise `session` falls out of scope and its drop closes it, which
        // is what tells the client the server has no room.

        Ok(())
    }

    /// Closes the session at `wait_index` and removes it from the table.
    ///
    /// The port occupying index zero cannot be dropped: a signalled port that
    /// reports itself closed means the port this server exists to host is gone,
    /// which ends the loop.
    fn drop_session(&mut self, wait_index: usize) -> Result<(), ServeError> {
        if wait_index == PORT_INDEX {
            return Err(ServeError::PortClosed);
        }

        let session_index = wait_index - 1;
        if session_index < self.sessions.len() {
            // Dropping the owner closes the handle.
            self.sessions.remove(session_index);
        }

        Ok(())
    }

    /// Decodes the message in `buf`, dispatches it, and writes the reply back
    /// into `buf`.
    fn answer(&mut self, buf: &mut nx_sys_thread_tls::IpcBuffer) -> Result<Answer, ServeError> {
        // The decoded request borrows the buffer, so the whole of the decoding
        // and the handler call happens inside this block: the reply cannot be
        // written until every borrow of the bytes it overwrites has ended.
        let action = match hipc::parse_request(buf.as_array()) {
            // The framing itself did not decode, so which protocol wrote the
            // message is exactly what is not known.
            Err(err) => Action::Refuse(CMIF_UNVERSIONED, err.to_rc()),
            Ok(frame) => match parse_request(&frame) {
                Ok(Inbound::Close) => Action::Close,
                Ok(Inbound::Control(request)) => {
                    // Control requests are the framework's to answer, and this
                    // crate hosts no domains, so every one is refused. A client
                    // reads that as an interface that will not convert.
                    Action::Refuse(request.protocol(), NOT_SUPPORTED)
                }
                Ok(Inbound::Command(request)) => {
                    let protocol = request.protocol();
                    Action::Answer(protocol, self.service.call(request))
                }
                // Same reasoning as the framing failure above: the message type
                // is what failed to name a protocol.
                Err(err) => Action::Refuse(CMIF_UNVERSIONED, err.to_rc()),
            },
        };

        // A reply that cannot be put on the wire still owes its sender an
        // answer, and a result code is the only thing this protocol has to
        // give one with. So a failure here becomes the code the next block
        // reports, never a reason to stop serving.
        let (protocol, failure) = match action {
            Action::Close => return Ok(Answer::Closed),
            Action::Refuse(protocol, result) => (protocol, result),
            Action::Answer(protocol, response) => match write_response(buf, protocol, response) {
                Ok(()) => return Ok(Answer::Replied),
                Err(result) => (protocol, result),
            },
        };

        write_failure(buf, protocol, failure)?;
        Ok(Answer::Replied)
    }
}

/// Encodes `response` for `protocol` and writes it into `buf`.
///
/// # Errors
///
/// Returns the result code to report in its place when the response cannot go
/// on the wire: pointer data the protocol has no descriptor for, or a body
/// larger than a message can carry. Both are the handler's reply being wrong
/// rather than the server being broken, so the code goes back to the client
/// that asked and `buf` is left for [`write_failure`] to fill.
fn write_response<B: HipcPayload>(
    buf: &mut nx_sys_thread_tls::IpcBuffer,
    protocol: Protocol,
    response: Response<B>,
) -> Result<(), crate::error::ResultCode> {
    let reply = response.into_reply(protocol).map_err(|err| err.to_rc())?;
    reply
        .write_to(buf.as_array_mut())
        .map_err(|err| err.to_rc())
}

/// Writes a reply carrying `result` and nothing else.
///
/// # Errors
///
/// Returns [`ServeError`] if even this reply cannot be produced. Both paths are
/// unreachable in practice - the reply carries no descriptors, which is the
/// only thing a protocol refuses, and no body, which is the only thing that
/// overflows the buffer - but the encoder's signature admits them, and a server
/// that assumed them away would be assuming away the one path that reports
/// every other failure.
fn write_failure(
    buf: &mut nx_sys_thread_tls::IpcBuffer,
    protocol: Protocol,
    result: crate::error::ResultCode,
) -> Result<(), ServeError> {
    let reply = Response::new(result)
        .into_reply(protocol)
        .map_err(ServeError::EncodeFailureReply)?;
    reply
        .write_to(buf.as_array_mut())
        .map_err(ServeError::WriteFailureReply)
}

/// Error returned by [`Server::run`].
#[derive(Debug, thiserror::Error)]
pub enum ServeError {
    /// The kernel refused the wait.
    ///
    /// Covers every failure of the reply-and-receive call that is not a session
    /// closing, a cancellation, or termination, each of which the loop handles
    /// itself. The wait set is left as it was.
    #[error("Failed to wait on the port and its sessions")]
    Wait(#[source] ReplyAndReceiveError),
    /// The port itself reported closed.
    ///
    /// The port a server exists to host cannot be replaced, so the loop ends.
    /// Sessions still open are closed as the server is dropped.
    #[error("The hosted port was closed")]
    PortClosed,
    /// The kernel refused to hand over a session the port had pending.
    #[error("Failed to accept a session on the port")]
    Accept(#[source] nx_svc::ipc::AcceptSessionError),
    /// A reply carrying nothing but a result code could not be encoded for the
    /// session's protocol.
    ///
    /// Occurs only on the path that reports every other reply failure, so it
    /// means the server can no longer tell a client anything. A handler's own
    /// reply being unencodable is not this: that is reported to its sender as a
    /// result code and the loop continues.
    #[error("Failed to encode even a bare failure reply")]
    EncodeFailureReply(#[source] super::response::PointerDataOverTipcError),
    /// A reply carrying nothing but a result code did not fit the IPC buffer.
    ///
    /// Occurs on the same path and means the same thing as
    /// [`EncodeFailureReply`](Self::EncodeFailureReply): a handler returning
    /// more than a message can carry is answered with a result code instead,
    /// not reported here.
    #[error("A bare failure reply does not fit the IPC buffer")]
    WriteFailureReply(#[source] hipc::WriteError),
}

/// Result code reported for a request this crate declines to serve.
///
/// Control requests and unsupported protocol shapes both land here: the command
/// named something the running implementation does not provide.
const NOT_SUPPORTED: crate::error::ResultCode = libnx_error(LibnxError::IncompatSysVer);

/// Protocol used to answer a request that did not decode.
///
/// A message whose type named no protocol has no protocol to answer in either.
/// CMIF with no context token is the shape a client reads a bare result code
/// out of, whichever it wrote.
const CMIF_UNVERSIONED: Protocol = Protocol::Cmif(CmifVersion::Plain);

/// What answering one message left the session in.
enum Answer {
    /// A reply is in the IPC buffer, waiting for the next syscall to send it.
    Replied,
    /// The client asked to close the session; nothing is replied.
    Closed,
}

/// What the decoded message asks the loop to do.
///
/// Exists so the decoding, which borrows the IPC buffer, finishes before the
/// reply is written into those same bytes. Every variant is owned.
#[expect(
    clippy::large_enum_variant,
    reason = "the value is a local that lives across one match in `answer`; boxing it would put an \
              allocation on the path every request takes, to save stack the frame already has"
)]
enum Action<B: HipcPayload> {
    /// Close the session without replying.
    Close,
    /// Answer with a bare result code.
    Refuse(Protocol, crate::error::ResultCode),
    /// Answer with what the service produced.
    Answer(Protocol, Response<B>),
}
