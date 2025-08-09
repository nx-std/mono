//! Thread Inner Implementations

use nx_svc::thread::Handle;

use crate::thread_stackmem::ThreadStackMem;

/// Inner thread data that will be wrapped in Arc<Pin<_>>
pub(crate) struct ThreadInner {
    /// Nintendo Switch specific thread handle
    pub handle: Handle,

    /// Stack memory information
    pub stack_mem: ThreadStackMem,
}
