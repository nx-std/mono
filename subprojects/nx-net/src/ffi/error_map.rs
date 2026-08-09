//! Maps the resolver's idiomatic Rust errors onto the C result codes a
//! caller expects.
//!
//! The musl-shaped resolver classifies a failure into a `thiserror` enum
//! variant; turning that classification into the numeric `errno`,
//! `h_errno`, and `EAI_*` / `h_errno`-string values a C caller expects is a
//! C-ABI concern, so it lives here under the `ffi` feature rather than in the
//! resolver itself. The `__nx_net__*` exports read these codes straight off
//! the resolver's error types.

use core::ffi::{
    CStr,
    c_int,
};

use nx_service_sfdnsres::CommandError;
use nx_sf::cmif::SendError;
use nx_svc::{
    error::Module,
    ipc::SendSyncError,
    result::Error as ResultError,
};

use crate::{
    ffi::abi,
    resolve::resolver::{
        HostLookupError,
        NameInfoError,
        ResolveError,
    },
};

impl ResolveError {
    /// The C `errno` value for this failure.
    pub fn errno(&self) -> c_int {
        match self {
            Self::Ipc(err) => command_errno(err),
            // The resolver supplied its own condition; the C code comes back
            // out here, where the numbering is the caller's concern.
            Self::Resolver(failure) => failure.errno.to_raw() as c_int,
        }
    }

    /// The `getaddrinfo` `EAI_*` return code for this failure.
    pub fn gai_code(&self) -> c_int {
        match self {
            // An IPC transport or decode failure is a system error; `errno`
            // carries the specific cause.
            Self::Ipc(_) => abi::EAI_SYSTEM,
            Self::Resolver(failure) => failure.kind.to_wire(),
        }
    }
}

impl HostLookupError {
    /// The C `errno` value for this failure.
    pub fn errno(&self) -> c_int {
        match self {
            Self::ByName(err) => command_errno(err),
            Self::ByAddr(err) => command_errno(err),
            // The resolver supplied its own condition; the C code comes back
            // out here, where the numbering is the caller's concern.
            Self::Resolver(failure) => failure.errno.to_raw() as c_int,
        }
    }

    /// The C `h_errno` value for this failure.
    pub fn h_errno(&self) -> c_int {
        match self {
            Self::Resolver(failure) => failure.kind.to_wire(),
            // The other failure paths have no resolver `h_errno`; the C
            // resolver reports them as an internal error and carries the
            // specific cause in [`errno`](Self::errno).
            Self::ByName(_) | Self::ByAddr(_) => abi::NETDB_INTERNAL,
        }
    }
}

impl NameInfoError {
    /// The C `errno` value for this failure.
    pub fn errno(&self) -> c_int {
        match self {
            Self::Ipc(err) => command_errno(err),
            // The resolver supplied its own condition; the C code comes back
            // out here, where the numbering is the caller's concern.
            Self::Resolver(failure) => failure.errno.to_raw() as c_int,
        }
    }

    /// The `getnameinfo` `EAI_*` return code for this failure.
    pub fn gai_code(&self) -> c_int {
        match self {
            // An IPC transport failure is a system error; `errno` carries the
            // specific cause.
            Self::Ipc(_) => abi::EAI_SYSTEM,
            Self::Resolver(failure) => failure.kind.to_wire(),
        }
    }
}

/// Maps a Horizon IPC result-code failure to the C `errno` a caller reads.
///
/// The C resolver classifies a failed `sfdnsres` request by the Horizon module
/// that produced the result code: a service-manager failure is a transient
/// "try again", a kernel failure is a bad address, and anything else is an
/// unclassified broken pipe.
pub fn ipc_result_to_errno(err: ResultError) -> c_int {
    match err.module() {
        Ok(Module::SM) => abi::EAGAIN,
        Ok(Module::Kernel) => abi::EFAULT,
        // A module this build has no name for is no more classifiable than a
        // named one outside the two above, so both land here.
        Ok(_) | Err(_) => abi::EPIPE,
    }
}

/// Classifies an `nx-service-sfdnsres` command failure into an `errno`.
///
/// Every `sfdnsres` command shares the single [`CommandError`] type: it may
/// fail while it is sent, while its response is parsed, or while the
/// serialized wire format is decoded. Only a send that reached the kernel
/// carries a Horizon result code that can be classified; a response that could
/// not be parsed is reported as an unclassified broken pipe, and a malformed
/// wire format as an invalid-argument failure.
fn command_errno(err: &CommandError) -> c_int {
    match err {
        CommandError::SendRequest(send) => send_errno(send),
        CommandError::ParseResponse(_) => abi::EPIPE,
        CommandError::Decode(_) => abi::EINVAL,
    }
}

/// Classifies a request-send failure into an `errno`.
fn send_errno(err: &SendError) -> c_int {
    match err {
        // The request never left this process, so there is no result code to
        // classify: report it as the same unclassified failure a caller sees
        // when a reply cannot be parsed.
        SendError::Layout(_) => abi::EPIPE,
        SendError::SendRequest(err) => send_sync_errno(err),
    }
}

/// Classifies a kernel `SendSyncRequest` failure into an `errno`.
fn send_sync_errno(err: &SendSyncError) -> c_int {
    match err {
        // An unforeseen result code: classify it by its Horizon module.
        SendSyncError::Unknown(result) => ipc_result_to_errno(*result),
        // Every other variant is a kernel-module fault.
        SendSyncError::TerminationRequested
        | SendSyncError::OutOfResource
        | SendSyncError::InvalidHandle
        | SendSyncError::SessionClosed => abi::EFAULT,
    }
}

/// Returns the textual description of a `getaddrinfo` `EAI_*` error code.
///
/// Maps an `EAI_*` code to a fixed, NUL-terminated description. The strings
/// are compiled-in constants, so the result borrows for `'static` and needs
/// no service round-trip — unlike the C resolver, which queries `sfdnsres`
/// for the text. An unrecognized code maps to a generic "unknown error"
/// description.
pub fn gai_strerror(err: c_int) -> &'static CStr {
    match err {
        0 => c"Resolver succeeded",
        abi::EAI_ADDRFAMILY => c"Address family for hostname not supported",
        abi::EAI_AGAIN => c"Temporary failure in name resolution",
        abi::EAI_BADFLAGS => c"Bad value for ai_flags",
        abi::EAI_FAIL => c"Non-recoverable failure in name resolution",
        abi::EAI_FAMILY => c"ai_family not supported",
        abi::EAI_MEMORY => c"Memory allocation failure",
        abi::EAI_NODATA => c"No address associated with hostname",
        abi::EAI_NONAME => c"Name or service not known",
        abi::EAI_SERVICE => c"Service not supported for ai_socktype",
        abi::EAI_SOCKTYPE => c"ai_socktype not supported",
        abi::EAI_SYSTEM => c"System error",
        abi::EAI_BADHINTS => c"Invalid value for hints",
        abi::EAI_PROTOCOL => c"Resolved protocol is unknown",
        abi::EAI_OVERFLOW => c"Argument buffer overflow",
        _ => c"Unknown getaddrinfo error",
    }
}

/// Returns the textual description of a host-lookup `h_errno` error code.
///
/// Maps an `h_errno` value (set by the `gethostby*` family) to a fixed,
/// NUL-terminated description. As with [`gai_strerror`], the strings are
/// compiled-in constants so the result is `'static` and needs no service
/// round-trip. An unrecognized code maps to a generic "unknown error"
/// description.
pub fn hstrerror(err: c_int) -> &'static CStr {
    match err {
        abi::NETDB_INTERNAL => c"Resolver internal error",
        abi::NETDB_SUCCESS => c"Resolver error 0 (no error)",
        abi::HOST_NOT_FOUND => c"Unknown host",
        abi::TRY_AGAIN => c"Host name lookup failure",
        abi::NO_RECOVERY => c"Unknown server error",
        abi::NO_DATA => c"No address associated with name",
        _ => c"Unknown resolver error",
    }
}
