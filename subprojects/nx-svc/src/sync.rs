use super::raw::{__nx_svc_arbitrate_lock, __nx_svc_arbitrate_unlock};

// TODO: Document
pub const HANDLE_WAIT_MASK: u32 = 0x40000000;

/// Makes the calling thread wait until a specific memory address, `tag_location`, no longer
/// contains a given value, `self_tag`.
// TODO: Improve documentation
pub unsafe fn arbitrate_lock(
    wait_tag: u32,
    tag_location: *mut u32,
    self_tag: u32,
) -> Result<(), ()> {
    let res = unsafe { __nx_svc_arbitrate_lock(wait_tag, tag_location, self_tag) };
    if res == 0 { Ok(()) } else { Err(()) }
}

// TODO: Documen
pub unsafe fn arbitrate_unlock(tag_location: *mut u32) -> Result<(), ()> {
    let res = unsafe { __nx_svc_arbitrate_unlock(tag_location) };
    if res == 0 { Ok(()) } else { Err(()) }
}
