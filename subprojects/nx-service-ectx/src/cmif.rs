//! CMIF protocol operations for the error context service.

use nx_sf::{
    cmif,
    hipc::BufferMode,
    ipc::{self, Handle as SessionHandle},
};

use crate::proto;

/// Output from a [`pull_context`] call.
#[derive(Debug, Clone, Copy)]
pub struct PullContextOutput {
    /// Unknown output field.
    pub field0: i32,
    /// Total error context size.
    pub total_size: u32,
    /// Actual error context size written to the buffer.
    pub size: u32,
}

/// Pulls error context associated with a descriptor and result code.
pub fn pull_context(
    session: SessionHandle,
    dst: &mut [u8],
    descriptor: u32,
    result: u32,
) -> Result<PullContextOutput, PullContextError> {
    #[repr(C)]
    #[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
    struct Input {
        descriptor: u32,
        result: u32,
    }

    #[repr(C)]
    #[derive(Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
    struct Output {
        field0: i32,
        total_size: u32,
        size: u32,
    }

    let input = Input { descriptor, result };

    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::PULL_CONTEXT)
        .with_data_value(&input)
        .add_output_buffer_raw(dst.as_mut_ptr(), dst.len(), BufferMode::Normal)
        .build();
    req.write_to(&mut buf)
        .map_err(PullContextError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(PullContextError::SendRequest)?;

    let resp = cmif::parse_response::<&Output>(&buf).map_err(PullContextError::ParseResponse)?;

    let out = *resp.payload;

    Ok(PullContextOutput {
        field0: out.field0,
        total_size: out.total_size,
        size: out.size,
    })
}

/// Error returned by [`pull_context`].
#[derive(Debug, thiserror::Error)]
pub enum PullContextError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}
