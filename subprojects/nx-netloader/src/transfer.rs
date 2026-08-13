//! Receiving one program over an accepted connection.
//!
//! The steps run in the order the host drives them, and each one has to complete before the next
//! can begin, so this reads top to bottom as the exchange itself.
//!
//! # Why the reads do not wait on readiness first
//!
//! A read asks the socket directly and retries while it has nothing, rather than waiting for the
//! socket to be reported readable. On this console the two are not equivalent: a transfer that a
//! readiness check reports nothing for is one a receive returns bytes for, and a run of those ends
//! with the whole transfer abandoned over a name the host did send. The homebrew menu's own
//! netloader reads this way and receives what waiting did not, which is the strongest evidence
//! available that the waiting is what breaks rather than the reading.
//!
//! The cost is a loop that has to bound its own patience, so it watches the clock and sleeps
//! between attempts rather than spinning on a socket that will have something shortly.

use alloc::{
    string::{
        String,
        ToString as _,
    },
    vec,
};
use core::{
    fmt::Write as _,
    net::SocketAddr,
};

use miniz_oxide::{
    DataFormat,
    MZError,
    MZFlush,
    MZStatus,
    inflate::stream::{
        InflateState,
        inflate,
    },
};
use nx_std::fs::File;
use nx_std_path::Path;
use nx_sys_net::Socket;

/// How long one read waits before the transfer is given up on.
///
/// A host that stops mid-transfer would otherwise leave the runner blocked with no way back to the
/// screen short of rebooting the console.
const RECV_TIMEOUT_NS: u64 = 10_000_000_000;

/// How long to sleep between read attempts that found nothing.
const RETRY_PAUSE_NS: u64 = 1_000_000;

/// The largest compressed chunk the host is allowed to announce.
const CHUNK_SIZE: usize = 16 * 1024;

/// The largest command line the host is allowed to send.
const ARGS_SIZE: usize = 3072;

/// How much room the received program's path gets.
///
/// The runner hands this path to the process loader, which copies it into a buffer it owns without
/// checking how long it is. That buffer is 512 bytes, so staying under half of it leaves no way to
/// overrun it.
const PATH_SIZE: usize = 256;

/// What one receive attempt found.
pub enum Outcome {
    /// A program arrived and was written to the drop directory.
    Received {
        /// Where the program was written.
        path: String,
        /// The command line to launch it with, quoted the way the runtime parses it.
        cmdline: String,
    },
    /// A host connected but the transfer did not complete.
    Failed {
        /// Why it did not complete.
        reason: String,
    },
}

/// Receives one program over an accepted connection.
///
/// `extra_arg` is an argument to give the program ahead of the ones the host sent, or `None` for
/// none. It is how the caller says something to every program it launches, whatever the host had to
/// say.
///
/// Returns [`Outcome::Failed`] rather than an error for anything the host did, because a failed
/// transfer is an ordinary event the runner reports and carries on from.
pub fn receive(
    sock: &Socket,
    peer: SocketAddr,
    drop_dir: &str,
    extra_arg: Option<&str>,
    progress: &mut dyn FnMut(&str, usize, usize),
) -> Outcome {
    match run(sock, peer, drop_dir, extra_arg, progress) {
        Ok(outcome) => outcome,
        Err(err) => Outcome::Failed {
            reason: err.to_string(),
        },
    }
}

/// The exchange itself, with every step free to give up by returning an error.
fn run(
    sock: &Socket,
    peer: SocketAddr,
    drop_dir: &str,
    extra_arg: Option<&str>,
    progress: &mut dyn FnMut(&str, usize, usize),
) -> Result<Outcome, TransferError> {
    let name = recv_file_name(sock)?;
    let size = recv_u32(sock)?;

    // The host waits for a status word before sending anything else, and it waits whether or not
    // the file can be received here. So everything that decides that answer happens first, and its
    // failure becomes a code to send rather than a return: a transfer refused without a word sent
    // leaves the host waiting for one until its own patience runs out.
    let prepared = prepare(&name, drop_dir, size);
    send_response(sock, response_code(&prepared))?;
    let (path, partial) = prepared?;

    match receive_body(sock, &partial, &path, &name, size, progress) {
        Ok(()) => {}
        Err(err) => {
            // A part-written program would be launchable and wrong, which is worse than not having
            // it at all. The removal is best-effort: the transfer has already failed, and a drop
            // directory that will not give the file up is not something this can act on.
            let _ = nx_std::fs::remove_file(Path::new(&partial));
            return Ok(Outcome::Failed {
                reason: err.to_string(),
            });
        }
    }

    send_response(sock, 0)?;

    let cmdline = build_cmdline(sock, &path, extra_arg, peer)?;
    Ok(Outcome::Received { path, cmdline })
}

/// Decides where the program goes and takes the room for it.
///
/// Everything the status word reports on, in one place, so that the caller has a single result to
/// turn into that word.
///
/// Returns the path the program will end up at and the name it is built up under.
fn prepare(name: &str, drop_dir: &str, size: u32) -> Result<(String, String), TransferError> {
    let path = drop_path(name, drop_dir)?;
    // Everything up to the rename at the end happens under a name of its own, so that a transfer
    // which stops part-way leaves its wreckage somewhere the runner will never launch from.
    let partial = partial_path(&path)?;
    reserve(&partial, size)?;
    Ok((path, partial))
}

/// Reads the body, verifies it, and puts it in place under its own name.
fn receive_body(
    sock: &Socket,
    partial: &str,
    path: &str,
    name: &str,
    size: u32,
    progress: &mut dyn FnMut(&str, usize, usize),
) -> Result<(), TransferError> {
    let mut file = File::create(Path::new(partial)).map_err(TransferError::CreateFile)?;
    inflate_to_file(sock, &mut file, name, size as usize, progress)?;

    // What the writes produced is not on the card until the close says so: the last of it is still
    // held in a buffer, and a close that cannot place it is how a card that filled up part-way
    // through reports itself.
    file.close().map_err(TransferError::CloseFile)?;

    if !looks_like_program(partial)? {
        return Err(TransferError::NotAProgram);
    }

    // Everything that had to be true of it is true, so it takes its own name. Until this point
    // nothing existed that the runner would hand to the loader.
    //
    // The card is asked to let go of the previous one first: a rename here does not replace what it
    // lands on, it refuses, and the same suite arriving twice is the ordinary case rather than the
    // exception. Between the two calls the name is absent, which is the safe direction to fail in —
    // there is nothing to launch either way.
    //
    // The removal is expected to fail the first time a given program arrives, since there is nothing
    // to remove; only the rename's success is worth reporting.
    let _ = nx_std::fs::remove_file(Path::new(path));
    nx_std::fs::rename(Path::new(partial), Path::new(path)).map_err(TransferError::PutInPlace)
}

/// Inflates the chunk stream into `file` until the host's stream ends.
fn inflate_to_file(
    sock: &Socket,
    file: &mut File,
    name: &str,
    total: usize,
    progress: &mut dyn FnMut(&str, usize, usize),
) -> Result<(), TransferError> {
    let mut state = InflateState::new_boxed(DataFormat::Zlib);
    let mut compressed = vec![0u8; CHUNK_SIZE];
    let mut plain = vec![0u8; CHUNK_SIZE];
    let mut written = 0usize;

    loop {
        let announced = recv_u32(sock)? as usize;
        if announced > CHUNK_SIZE {
            return Err(TransferError::ChunkTooLarge {
                announced,
                limit: CHUNK_SIZE,
            });
        }

        recv_exact(sock, &mut compressed[..announced])?;

        // One chunk can inflate to more than the output buffer holds, so it is fed until the
        // decompressor has taken all of it -- and then once more with nothing left to give it,
        // because the end of the stream is a status it reports on a call of its own rather than
        // alongside the last bytes it consumed. Stopping as soon as the chunk was consumed would
        // leave that status unread and send the loop back to wait for a chunk the host already
        // finished sending.
        let mut taken = 0usize;
        loop {
            let result = inflate(
                &mut state,
                &compressed[taken..announced],
                &mut plain,
                MZFlush::None,
            );

            taken += result.bytes_consumed;
            let produced = result.bytes_written;

            if produced > 0 {
                file.write_all(&plain[..produced])
                    .map_err(TransferError::Write)?;
                written += produced;
                progress(name, written, total);
            }

            match result.status {
                // The stream is complete. Nothing on the wire says so, so this is the only place
                // the end of the transfer can be learned.
                Ok(MZStatus::StreamEnd) => return finish(written, total),
                Ok(_) => {}
                // No progress rather than damage: the decompressor has taken everything this chunk
                // carried and has nothing pending, which is what it reports when a chunk's output
                // happens to land exactly on the end of the buffer. The next chunk is what it is
                // waiting for, so this ends the pass over this one instead of ending the transfer.
                // Whether the host has more to send is settled by the read at the top of the outer
                // loop, which gives up if nothing comes.
                Err(MZError::Buf) => break,
                Err(err) => return Err(TransferError::Damaged(err)),
            }

            // The decompressor took nothing and produced nothing, so feeding it the same bytes
            // again would loop forever; what it is waiting for is the next chunk.
            if result.bytes_consumed == 0 && produced == 0 {
                break;
            }
        }
    }
}

/// Checks the length the host announced against what actually arrived.
///
/// A stream can end cleanly having carried fewer bytes than the host said it would, and every check
/// above would still have passed: the chunks were well formed and the stream ended properly, it was
/// simply short. Only the length the host announced says how much there should have been.
fn finish(written: usize, total: usize) -> Result<(), TransferError> {
    match written == total {
        true => Ok(()),
        false => Err(TransferError::ShortStream { written, total }),
    }
}

/// Whether the file at `path` begins the way a program does.
///
/// The name having ended in `.nro` said only what the host called it. This looks at what arrived:
/// the loader will refuse anything without this marker, and refusing it here costs a read of a few
/// bytes rather than a launch that fails on the console with nothing to say why.
fn looks_like_program(path: &str) -> Result<bool, TransferError> {
    /// Where the marker sits, past the branch and the header the loader reads first.
    const MAGIC_OFFSET: u64 = 0x10;
    /// What every program starts with, at that offset.
    const MAGIC: &[u8; 4] = b"NRO0";

    let mut file = File::open(Path::new(path)).map_err(TransferError::Reopen)?;
    file.seek(nx_std::fs::SeekFrom::Start(MAGIC_OFFSET))
        .map_err(TransferError::Reopen)?;

    let mut magic = [0u8; MAGIC.len()];
    let read = file.read(&mut magic).map_err(TransferError::Reopen)?;

    Ok(read == magic.len() && &magic == MAGIC)
}

/// Reads the name the host is sending, reduced to its last component.
///
/// The runner writes everything into one directory of its own, so only the file name itself is
/// kept: no name a host sends can place a file anywhere else.
fn recv_file_name(sock: &Socket) -> Result<String, TransferError> {
    let length = recv_u32(sock)? as usize;
    if length == 0 || length >= PATH_SIZE {
        return Err(TransferError::NameLength {
            length,
            limit: PATH_SIZE - 1,
        });
    }

    let mut sent = vec![0u8; length];
    recv_exact(sock, &mut sent)?;

    let sent = core::str::from_utf8(&sent).map_err(|_| TransferError::NameNotText)?;
    let base = match sent.rsplit_once('/') {
        Some((_, base)) => base,
        None => sent,
    };
    if base.is_empty() {
        return Err(TransferError::NameEndsInSeparator);
    }

    Ok(base.to_string())
}

/// Builds the path a program of this name will end up at.
///
/// `drop_dir` is the caller's, not this crate's: which directory a received program belongs in is a
/// policy of the program embedding the protocol, and every program it receives goes there under the
/// file name alone, so nothing a host sends can name a file outside it.
fn drop_path(name: &str, drop_dir: &str) -> Result<String, TransferError> {
    if !name.to_ascii_lowercase().ends_with(".nro") {
        return Err(TransferError::NotAProgram);
    }

    // The directory is created on the way: a card that has never run a test has no such directory,
    // and it already existing is the ordinary case rather than a failure.
    let _ = nx_std::fs::create_dir(Path::new(drop_dir));

    let mut path = String::new();
    write!(&mut path, "{drop_dir}/{name}").map_err(|_| TransferError::PathTooLong)?;
    match path.len() < PATH_SIZE {
        true => Ok(path),
        false => Err(TransferError::PathTooLong),
    }
}

/// Names the file a program is written to while it is still arriving.
///
/// A program is built up under this name and only takes its own once every byte of it is there, so
/// nothing that stops part-way — a host that goes away, a console that loses power — can leave
/// something launchable behind. Whatever is found under this name is wreckage from a transfer that
/// did not finish.
fn partial_path(path: &str) -> Result<String, TransferError> {
    let mut partial = String::new();
    write!(&mut partial, "{path}.part").map_err(|_| TransferError::PathTooLong)?;
    Ok(partial)
}

/// Takes the room the program will need.
///
/// Growing the file to its final length asks the card for the room up front, so a card without it
/// fails here rather than part-way through the transfer.
fn reserve(partial: &str, size: u32) -> Result<(), TransferError> {
    let mut file = File::create(Path::new(partial)).map_err(TransferError::CreateFile)?;

    match file.set_len(u64::from(size)) {
        Ok(()) => Ok(()),
        Err(err) => {
            // The reservation is what failed, so the file it left behind is of no use to anything.
            let _ = nx_std::fs::remove_file(Path::new(partial));
            Err(TransferError::NotEnoughSpace(err))
        }
    }
}

/// The status word that says what stopped a transfer before it started.
///
/// The host reads these and reports them, so the three the protocol defines are sent as themselves
/// rather than collapsed into one refusal.
fn response_code(prepared: &Result<(String, String), TransferError>) -> i32 {
    match prepared {
        Ok(_) => 0,
        Err(TransferError::CreateFile(_)) => -1,
        Err(TransferError::NotEnoughSpace(_)) => -2,
        Err(TransferError::NotAProgram) => -3,
        // Anything else stopped before a file was involved; the host only distinguishes the three
        // above, and treats the rest as a refusal it cannot act on.
        Err(_) => -1,
    }
}

/// Reads the arguments the host sent and builds the command line.
///
/// The line starts with the program's own path, the way a loader is expected to pass it, and ends
/// with the token the runtime reads the host's address out of: a program that finds it there can
/// send its output back to the host instead of only to the screen. The token has to be last, so the
/// arguments the host sent go in between.
fn build_cmdline(
    sock: &Socket,
    path: &str,
    extra_arg: Option<&str>,
    peer: SocketAddr,
) -> Result<String, TransferError> {
    let length = recv_u32(sock)? as usize;
    if length > ARGS_SIZE {
        return Err(TransferError::ArgsTooLong {
            length,
            limit: ARGS_SIZE,
        });
    }

    let mut args = vec![0u8; length];
    if length > 0 {
        recv_exact(sock, &mut args)?;
    }

    let mut cmdline = String::new();
    append_arg(&mut cmdline, path);
    if let Some(extra) = extra_arg {
        append_arg(&mut cmdline, extra);
    }

    for arg in args.split(|&byte| byte == 0) {
        // Either the padding past the last argument, or a final one the host did not terminate;
        // there is nothing more to read either way.
        if arg.is_empty() {
            break;
        }
        // An argument that is not text is not something the runtime can be handed; the rest of the
        // line is still worth having.
        let Ok(arg) = core::str::from_utf8(arg) else {
            break;
        };
        append_arg(&mut cmdline, arg);
    }

    // Bare, unlike every argument before it. The runtime reads this token off the raw line rather
    // than out of the parsed arguments, and looks for the last whitespace-delimited word to be the
    // address and the marker and nothing else; a pair of quotes around it is two characters too
    // many and the host goes unrecorded.
    if let SocketAddr::V4(peer) = peer {
        // The address goes on the wire in the layout the runtime reads it back in, which is the
        // one the octets are already in.
        let raw = u32::from_le_bytes(peer.ip().octets());
        // Writing into a `String` cannot fail, and a line missing its token still launches, so a
        // failure here is not worth failing the transfer over.
        let _ = write!(&mut cmdline, " {raw:08x}_NXLINK_");
    }

    Ok(cmdline)
}

/// Appends one argument to the command line, quoted.
///
/// The runtime that parses this line splits on spaces and honours double quotes, so quoting every
/// argument keeps one holding a space in one piece.
fn append_arg(cmdline: &mut String, arg: &str) {
    if !cmdline.is_empty() {
        cmdline.push(' ');
    }
    cmdline.push('"');
    cmdline.push_str(arg);
    cmdline.push('"');
}

/// Reads a little-endian length or status word.
fn recv_u32(sock: &Socket) -> Result<u32, TransferError> {
    let mut word = [0u8; 4];
    recv_exact(sock, &mut word)?;
    Ok(u32::from_le_bytes(word))
}

/// Writes a status word the host is waiting for.
fn send_response(sock: &Socket, response: i32) -> Result<(), TransferError> {
    let word = response.to_le_bytes();
    let mut sent = 0usize;

    while sent < word.len() {
        match sock.send(&word[sent..]) {
            Ok(0) => return Err(TransferError::HostStoppedListening),
            Ok(taken) => sent += taken,
            Err(err) if err.is_would_block() => nx_svc::thread::sleep(RETRY_PAUSE_NS),
            Err(err) => return Err(TransferError::Send(err)),
        }
    }

    Ok(())
}

/// Reads exactly enough bytes to fill `buf`, or gives up.
fn recv_exact(sock: &Socket, buf: &mut [u8]) -> Result<(), TransferError> {
    let wanted = buf.len();
    let deadline = Deadline::in_ns(RECV_TIMEOUT_NS);
    let mut filled = 0usize;

    while filled < wanted {
        match sock.recv(&mut buf[filled..]) {
            // The host closed the connection with the transfer unfinished. Told apart from a
            // malformed message because the two point at opposite ends of the problem: this one
            // says the host is not the one still talking to us.
            Ok(0) => {
                return Err(TransferError::HostClosed { filled, wanted });
            }
            Ok(received) => filled += received,
            Err(err) if err.is_would_block() => {
                if deadline.passed() {
                    return Err(TransferError::Quiet { filled, wanted });
                }
                nx_svc::thread::sleep(RETRY_PAUSE_NS);
            }
            Err(err) => return Err(TransferError::Recv(err)),
        }
    }

    Ok(())
}

/// A point in the future, on the counter that does not move when the clock is set.
struct Deadline {
    /// The tick the deadline expires at.
    at: u64,
}

impl Deadline {
    /// Builds a deadline `ns` nanoseconds from now.
    fn in_ns(ns: u64) -> Self {
        const NS_PER_SEC: u64 = 1_000_000_000;

        let frequency = nx_cpu::counter::frequency().to_raw();
        // The counter runs at tens of megahertz, so the product stays far below what a `u64` holds
        // for any deadline this crate sets.
        let span = ns / NS_PER_SEC * frequency + (ns % NS_PER_SEC) * frequency / NS_PER_SEC;

        Self {
            at: nx_cpu::counter::ticks().to_raw().wrapping_add(span),
        }
    }

    /// Whether the deadline has passed.
    fn passed(&self) -> bool {
        nx_cpu::counter::ticks().to_raw() >= self.at
    }
}

/// Errors returned while receiving a program.
#[derive(Debug, thiserror::Error)]
pub enum TransferError {
    /// The host closed the connection part-way through a message
    #[error("the host closed the connection, {filled} of {wanted} bytes in")]
    HostClosed {
        /// How much had arrived.
        filled: usize,
        /// How much was expected.
        wanted: usize,
    },

    /// Nothing arrived for long enough that the transfer was given up on
    #[error("nothing arrived in time, {filled} of {wanted} bytes in")]
    Quiet {
        /// How much had arrived.
        filled: usize,
        /// How much was expected.
        wanted: usize,
    },

    /// The connection could not be read
    #[error("the connection could not be read")]
    Recv(#[source] nx_sys_net::Error),

    /// The connection could not be written
    #[error("the connection could not be written")]
    Send(#[source] nx_sys_net::Error),

    /// The host stopped reading before the answer was sent
    #[error("the host stopped listening")]
    HostStoppedListening,

    /// The name the host sent is not a length this accepts
    #[error("the name is {length} bytes, which is not between 1 and {limit}")]
    NameLength {
        /// The length the host announced.
        length: usize,
        /// The largest this accepts.
        limit: usize,
    },

    /// The name the host sent is not text
    #[error("the name is not text")]
    NameNotText,

    /// The name ends in a separator, so it names no file
    #[error("the name ends in a path separator, so it names no file")]
    NameEndsInSeparator,

    /// The name does not end in the extension a program has
    #[error("what the host sent is not a program")]
    NotAProgram,

    /// The path the program would be written to does not fit
    #[error("the path does not fit")]
    PathTooLong,

    /// The drop directory would not take the file
    #[error("the file could not be created")]
    CreateFile(#[source] nx_std::fs::Error),

    /// The card had no room for the program
    #[error("the card has no room for it")]
    NotEnoughSpace(#[source] nx_std::fs::Error),

    /// A chunk larger than the ceiling was announced
    #[error("the host announced a {announced} byte chunk, over the {limit} allowed")]
    ChunkTooLarge {
        /// What the host announced.
        announced: usize,
        /// The largest this accepts.
        limit: usize,
    },

    /// The compressed stream is damaged
    #[error("the compressed stream is damaged")]
    Damaged(miniz_oxide::MZError),

    /// The stream ended cleanly but carried less than the host announced
    #[error("the program is {written} bytes of the {total} the host announced")]
    ShortStream {
        /// How much arrived.
        written: usize,
        /// How much was announced.
        total: usize,
    },

    /// The card would not take what was written
    #[error("the card would not take it")]
    Write(#[source] nx_std::fs::Error),

    /// The card would not take the last of what was written
    #[error("the card would not take the last of it")]
    CloseFile(#[source] nx_std::fs::Error),

    /// What arrived could not be read back to be checked
    #[error("what arrived could not be read back")]
    Reopen(#[source] nx_std::fs::Error),

    /// The program could not be put under its own name
    #[error("it could not be put in place")]
    PutInPlace(#[source] nx_std::fs::Error),

    /// The command line the host sent is longer than the ceiling
    #[error("the command line is {length} bytes, over the {limit} allowed")]
    ArgsTooLong {
        /// The length the host announced.
        length: usize,
        /// The largest this accepts.
        limit: usize,
    },
}
