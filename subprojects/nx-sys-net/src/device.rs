//! The `soc` device, and the map from a process descriptor to a service descriptor.
//!
//! A socket is reachable two ways, and both arrive here.
//!
//! The first is as an ordinary descriptor. `read(fd, …)`, `write(fd, …)` and `close(fd)` know
//! nothing about sockets: the C standard library looks the number up in [`nx_sys_fd`]'s table and
//! dispatches into whatever file backs it. Registering [`SocDevice`] is what makes that file a
//! [`SocketFile`], and implementing [`File`] on it is what makes those three calls work with no
//! socket-specific C code at all.
//!
//! The second is as a socket. `send`, `bind`, `listen` and the rest are not descriptor operations
//! and the table has no dispatch for them; they take a number and need the service's own
//! descriptor for it. [`sock_of`] is that lookup, and it is the only place in the crate that
//! performs it.
//!
//! ## Why the lookup copies the descriptor out
//!
//! [`nx_sys_fd::table::with_file`] holds the file's lock while it runs, and that lock serializes
//! every operation on the same descriptor. A socket must not be serialized that way: a server
//! blocked in `recv` on a connection while another thread `send`s on it is ordinary, and holding
//! the lock across the receive would deadlock the send behind it.
//!
//! So the lookup copies the service descriptor out and releases the lock, and the command runs
//! unlocked. The descriptor is a number the service owns; it stays valid because the [`Socket`]
//! holding it lives in the table until the descriptor is closed.

use alloc::boxed::Box;
use core::ffi::CStr;

use nx_service_bsd::{
    BsdSockFd,
    CommandError,
    PosixError,
};
use nx_sys_fd::{
    device::{
        Device,
        DeviceError,
        File,
        OpenFlags,
    },
    registry,
    table::{
        self,
        Fd,
    },
};

use crate::{
    session,
    socket::Socket,
};

/// The name paths resolve through, and the name the registry knows this device by.
const DEVICE_NAME: &CStr = c"soc";

/// The device every socket descriptor is opened against.
///
/// Holds nothing: a socket's state is the service descriptor inside its [`SocketFile`], and the
/// session every operation needs is process-wide. So one value serves the whole process and it is
/// a constant.
pub struct SocDevice;

/// The registered instance, which the registry holds a `&'static` to.
static SOC_DEVICE: SocDevice = SocDevice;

impl Device for SocDevice {
    fn name(&self) -> &'static CStr {
        DEVICE_NAME
    }

    /// Opens a path in the service's own namespace.
    ///
    /// The service has a small namespace of its own — the packet filter, and the like — that is
    /// reached by path rather than by protocol. An ordinary socket does not come through here; it
    /// comes from `socket()`, which has no path to resolve.
    fn open(&self, path: &CStr, flags: OpenFlags) -> Result<Box<dyn File>, DeviceError> {
        let sock = session::with_service(|svc| svc.open(path, open_flags_to_wire(flags)))
            .map_err(|_| DeviceError::Io)?
            .map_err(to_device_error)?;

        Ok(Box::new(SocketFile {
            sock: Socket::from_raw_unchecked(sock),
        }))
    }
}

/// Registers the device, so that descriptors can be opened against it.
///
/// # Errors
///
/// Returns [`RegisterFailed`] when the registry has no free slot. Registering twice replaces the
/// entry in place rather than taking a second slot, so this is not how a double initialization is
/// caught; the driver checks that separately.
pub fn register() -> Result<(), RegisterFailed> {
    registry::register(&SOC_DEVICE).map_err(|_| RegisterFailed)?;
    Ok(())
}

/// Error returned by [`register`] when the device registry is full.
///
/// Nothing was registered and no slot was disturbed.
#[derive(Debug, thiserror::Error)]
#[error("The device registry has no free slot for the socket device")]
pub struct RegisterFailed;

/// Unregisters the device.
///
/// Descriptors already open against it keep working, because each owns its own [`SocketFile`] and
/// reaches the service through the process-wide session rather than through the registry. What
/// stops working is opening new ones by path. Does nothing when the device is not registered.
pub fn unregister() {
    if let Some(id) = registry::find_by_name(DEVICE_NAME) {
        registry::unregister(id);
    }
}

/// Whether the device is registered.
pub fn is_registered() -> bool {
    registry::find_by_name(DEVICE_NAME).is_some()
}

/// Takes a descriptor for `sock`, handing the table the obligation to close it.
///
/// This is the second half of every call that produces a socket: the service issued a descriptor,
/// and the caller is owed one from the process's own table.
///
/// # Errors
///
/// Returns [`AdoptFailed::NotRegistered`] when the socket device is not registered, and
/// [`AdoptFailed::NoDescriptors`] when the table is full. `sock` is closed on either failure,
/// since nothing else has taken it on.
pub fn adopt(sock: Socket) -> Result<Fd, AdoptFailed> {
    let Some(device) = registry::find_by_name(DEVICE_NAME) else {
        return Err(AdoptFailed::NotRegistered);
    };

    let fd = table::open(device).map_err(|_| AdoptFailed::NoDescriptors)?;

    // The descriptor exists but owns nothing yet, so a failure here has to release it by hand.
    // `attach` rejects only a descriptor that is closed or already carries a file, and this one was
    // just taken from the table and given nothing, so neither can happen.
    if table::attach(fd, Box::new(SocketFile { sock })).is_err() {
        let _ = table::close(fd);
        return Err(AdoptFailed::NoDescriptors);
    }

    Ok(fd)
}

/// Errors returned by [`adopt`].
#[derive(Debug, thiserror::Error)]
pub enum AdoptFailed {
    /// The socket device is not registered
    ///
    /// Occurs when the driver was never initialized, or has exited. The socket was closed.
    #[error("The socket device is not registered")]
    NotRegistered,

    /// The descriptor table is full
    ///
    /// The socket was closed, so nothing leaked; the caller has no descriptor to return.
    #[error("No free descriptors remain")]
    NoDescriptors,
}

/// Returns the service descriptor behind the process descriptor `fd`.
///
/// # Errors
///
/// Returns [`LookupError::BadDescriptor`] when `fd` names nothing open, and
/// [`LookupError::NotASocket`] when it is open but backed by something else. The two are kept
/// apart because C reports them as different error numbers and callers branch on which they got.
pub fn sock_of(fd: Fd) -> Result<BsdSockFd, LookupError> {
    // The closure copies the descriptor out and returns, so the file's lock is released before the
    // command that uses it runs. See the module documentation for why that matters.
    let found = table::with_file(fd, |file| {
        (file as &dyn core::any::Any)
            .downcast_ref::<SocketFile>()
            .map(|socket_file| socket_file.sock.service_fd())
    });

    match found {
        Ok(Some(sock)) => Ok(sock),
        // Open, but what backs it is not a socket: either another device's file, or a stream
        // descriptor that owns no file at all.
        Ok(None) | Err(table::WithFileError::NotAFile) => Err(LookupError::NotASocket),
        Err(table::WithFileError::BadDescriptor) => Err(LookupError::BadDescriptor),
    }
}

/// Errors returned by [`sock_of`].
///
/// Nothing was looked up beyond the descriptor table, and nothing was sent.
#[derive(Debug, thiserror::Error)]
pub enum LookupError {
    /// The descriptor is not open
    #[error("The descriptor is not open")]
    BadDescriptor,

    /// The descriptor is open, but does not name a socket
    #[error("The descriptor does not name a socket")]
    NotASocket,
}

/// One open socket, as the descriptor table holds it.
///
/// Owns the [`Socket`], which is what closes the service descriptor when the process descriptor is
/// closed. [`File::close`] is deliberately left at its default: the release is unconditional and
/// has nothing to report that a caller could act on, which is exactly the case the trait says to
/// handle in [`Drop`].
pub struct SocketFile {
    sock: Socket,
}

impl File for SocketFile {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, DeviceError> {
        let sock = self.sock.service_fd();
        session::with_service(|svc| svc.read(sock, buf))
            .map_err(|_| DeviceError::NotConnected)?
            .map_err(to_device_error)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, DeviceError> {
        let sock = self.sock.service_fd();
        session::with_service(|svc| svc.write(sock, buf))
            .map_err(|_| DeviceError::NotConnected)?
            .map_err(to_device_error)
    }
}

/// Converts a failed command into the condition the descriptor table reports.
///
/// The table's vocabulary is narrower than the service's, so several conditions collapse into
/// [`DeviceError::Io`]. That is a loss only on this path: `read` and `write` are the two calls a C
/// program reaches through the descriptor table, and every other socket call goes through this
/// crate's own C surface, which keeps the condition and produces the exact error number for it.
pub(crate) fn to_device_error(err: CommandError) -> DeviceError {
    let CommandError::Service { source, .. } = err else {
        return DeviceError::Io;
    };

    match source {
        PosixError::WouldBlock => DeviceError::WouldBlock,
        PosixError::Interrupted => DeviceError::Interrupted,
        PosixError::ConnectionReset | PosixError::BrokenPipe => DeviceError::ConnectionReset,
        PosixError::NotConnected => DeviceError::NotConnected,
        PosixError::PermissionDenied | PosixError::NotPermitted => DeviceError::PermissionDenied,
        PosixError::TimedOut => DeviceError::TimedOut,
        PosixError::NotFound => DeviceError::NotFound,
        _ => DeviceError::Io,
    }
}

/// Rebuilds the C `open` flag word the service reads.
///
/// The descriptor table decodes the word before a device sees it, so that no device has to know
/// the bit values. This device does, because the service takes the word itself rather than a
/// decoded form, so the decoding has to be undone here.
///
/// The round trip is lossy in one direction that matters: `O_NONBLOCK` is not a flag
/// [`OpenFlags`] carries, so a path opened non-blocking arrives here as a blocking open. A caller
/// that needs it sets it afterwards through `fcntl`, which is the same thing the service does
/// internally.
fn open_flags_to_wire(flags: OpenFlags) -> i32 {
    /// Append: every write goes to the end.
    const O_APPEND: i32 = 0x0008;
    /// Create the entry when it does not exist.
    const O_CREAT: i32 = 0x0200;
    /// Discard the existing contents.
    const O_TRUNC: i32 = 0x0400;
    /// Fail the create when the entry exists.
    const O_EXCL: i32 = 0x0800;

    // The access mode is a value rather than a bit set: 0, 1 and 2 for read, write and both.
    let mut word = match (flags.read, flags.write) {
        (true, true) => 2,
        (false, true) => 1,
        _ => 0,
    };

    if flags.append {
        word |= O_APPEND;
    }
    if flags.create {
        word |= O_CREAT;
    }
    if flags.truncate {
        word |= O_TRUNC;
    }
    if flags.exclusive {
        word |= O_EXCL;
    }

    word
}
