//! `ISaveDataInfoReader` commands.
//!
//! The reader C holds is a domain object id like any other, so the entry points
//! here follow the same pattern as `IDirectory`: rebuild the wrapper around the
//! stored id for one command, hand the close obligation straight back, and let
//! only `fsSaveDataInfoReaderClose` discharge it.

use nx_service_fs::{
    FsSaveDataInfoReader,
    FsService,
    SaveDataFilter,
    SaveDataInfo,
    SaveDataSpaceId,
};
use nx_sf::{
    error::ToResultCode as _,
    ffi::Service,
    service::DispatchError,
};

use super::support::{
    object_id_of,
    sub_object_view,
};
use crate::{
    ffi::common::GENERIC_ERROR,
    services::fs,
};

/// Opens a reader over the savedata in one space.
///
/// Corresponds to `fsOpenSaveDataInfoReader()` in libnx.
///
/// # Safety
///
/// `out` must be null or writable. On success the caller owes the returned
/// reader a `fsSaveDataInfoReaderClose`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_open_save_data_info_reader(
    out: *mut Service,
    save_data_space_id: i32,
) -> u32 {
    let Ok(space_id) = SaveDataSpaceId::try_from(save_data_space_id) else {
        return GENERIC_ERROR;
    };
    // SAFETY: the caller guarantees `out` is null or writable.
    let Some(out) = (unsafe { out.as_mut() }) else {
        return GENERIC_ERROR;
    };

    open_reader(out, |service| service.open_save_data_info_reader(space_id))
}

/// Opens a reader over the savedata in one space that a filter admits.
///
/// Corresponds to `fsOpenSaveDataInfoReaderWithFilter()` in libnx.
///
/// # Safety
///
/// `out` must be null or writable, and `save_data_filter` must be null or point
/// to a readable `FsSaveDataFilter`. On success the caller owes the returned
/// reader a `fsSaveDataInfoReaderClose`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_open_save_data_info_reader_with_filter(
    out: *mut Service,
    save_data_space_id: i32,
    save_data_filter: *const SaveDataFilter,
) -> u32 {
    let Ok(space_id) = SaveDataSpaceId::try_from(save_data_space_id) else {
        return GENERIC_ERROR;
    };
    // SAFETY: the caller guarantees `save_data_filter` is null or points to a
    // readable filter.
    let Some(filter) = (unsafe { save_data_filter.as_ref() }) else {
        return GENERIC_ERROR;
    };
    // SAFETY: the caller guarantees `out` is null or writable.
    let Some(out) = (unsafe { out.as_mut() }) else {
        return GENERIC_ERROR;
    };

    open_reader(out, |service| {
        service.open_save_data_info_reader_with_filter(space_id, filter)
    })
}

/// Reads the next batch of savedata entries.
///
/// Corresponds to `fsSaveDataInfoReaderRead()` in libnx. A reader that has run
/// out reports zero entries rather than failing, which is what ends a caller's
/// walk.
///
/// # Safety
///
/// `s` must be null or point to a `Service` this module handed out, `buf` must
/// be writable for `max_entries` entries, and `total_entries` must be null or
/// writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_save_data_info_reader_read(
    s: *mut Service,
    buf: *mut SaveDataInfo,
    max_entries: usize,
    total_entries: *mut i64,
) -> u32 {
    if buf.is_null() {
        return GENERIC_ERROR;
    }
    // SAFETY: the caller guarantees a readable `Service`.
    let Some(object_id) = (unsafe { object_id_of(s) }) else {
        return GENERIC_ERROR;
    };

    // SAFETY: the caller guarantees `buf` holds `max_entries` entries.
    let entries = unsafe { core::slice::from_raw_parts_mut(buf, max_entries) };

    // `total_entries` is converted here rather than in the prologue above, unlike
    // every other entry point in this module. Nothing stops C from pointing it
    // inside `buf`, and a `&mut i64` minted before the read would then alias the
    // `&mut [SaveDataInfo]` the read borrows.
    match with_reader(object_id, |reader| reader.read(entries)) {
        Ok(count) => {
            // SAFETY: the caller guarantees `total_entries` is null or writable.
            if let Some(total_entries) = unsafe { total_entries.as_mut() } {
                *total_entries = count;
            }
            0
        }
        Err(rc) => rc,
    }
}

/// Closes a reader.
///
/// Corresponds to `fsSaveDataInfoReaderClose()` in libnx.
///
/// # Safety
///
/// `s` must be null or point to a writable `Service` this module handed out,
/// and must be closed exactly once: a second call would close an object id the
/// server may have since reissued.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_save_data_info_reader_close(s: *mut Service) {
    // SAFETY: the caller guarantees a readable `Service`.
    let Some(object_id) = (unsafe { object_id_of(s) }) else {
        return;
    };
    let Some(service) = fs::get_service() else {
        return;
    };

    // SAFETY: as in `fsFsClose` - one close per open, discharged here.
    drop(FsSaveDataInfoReader::from_raw_object_id_unchecked(
        &service, object_id,
    ));

    // SAFETY: the caller guarantees `s` is writable.
    unsafe { (*s).object_id = 0 };
}

/// Runs an opener and writes the reader it returned into `out`.
///
/// The close obligation goes straight back to C, which discharges it with
/// `fsSaveDataInfoReaderClose`.
fn open_reader(
    out: &mut Service,
    open: impl FnOnce(&FsService) -> Result<FsSaveDataInfoReader<'_>, DispatchError>,
) -> u32 {
    let Some(service) = fs::get_service() else {
        return GENERIC_ERROR;
    };
    let session = service.session_handle().to_handle();

    match open(&service) {
        Ok(reader) => {
            let object_id = reader.into_raw_object_id();
            *out = sub_object_view(session, object_id);
            0
        }
        Err(err) => err.to_rc(),
    }
}

/// Runs one command against the reader `object_id` names. See
/// [`super::support::with_filesystem`].
///
/// `object_id` is the one [`object_id_of`] read out of the `Service` C holds,
/// so it names an object this module handed out.
fn with_reader<R>(
    object_id: u32,
    f: impl FnOnce(&FsSaveDataInfoReader<'_>) -> Result<R, DispatchError>,
) -> Result<R, u32> {
    let Some(service) = fs::get_service() else {
        return Err(GENERIC_ERROR);
    };

    // SAFETY: as in `with_filesystem`.
    let wrapper = FsSaveDataInfoReader::from_raw_object_id_unchecked(&service, object_id);
    let result = f(&wrapper);
    let _ = wrapper.into_raw_object_id();

    result.map_err(|err| err.to_rc())
}
