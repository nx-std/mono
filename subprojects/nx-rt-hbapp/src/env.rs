//! # Runtime Environment State (NRO)
//!
//! Parses the homebrew loader's ABI configuration block and populates the
//! kind-agnostic [runtime environment state][nx_rt_core::env] every other
//! consumer reads back.
//!
//! The homebrew loader hands an NRO a `ConfigEntry` array describing the heap
//! override, command-line arguments, service overrides, applet type, HOS
//! version, and the rest of the startup environment. [`setup`] walks that
//! array exactly once and fills `nx-rt-core`'s environment container.
//!
//! The array's format is [`nx_hbabi`]'s, not this module's: this is one of the
//! two sides of a handover, and the side that writes it is a loader. What is
//! left here is the mapping from a decoded [`Entry`] to the piece of runtime
//! state it sets.
//!
//! The read accessors and the HOS-version / main-thread submodules are
//! re-exported from [`nx_rt_core::env`] so callers continue to reach them
//! through `crate::env::*`.

use core::ptr::NonNull;

pub use nx_hbabi::{
    ConfigEntries,
    ConfigEntry,
    Entry,
};
pub use nx_rt_core::env::{
    AccountUid,
    AppletType,
    LoaderReturnFn,
    MAX_SERVICE_OVERRIDES,
    NextLoad,
    ServiceName,
    ServiceOverride,
    SyscallHints,
    applet_type,
    applet_workaround,
    argv,
    exit_func_ptr,
    has_next_load,
    heap_override,
    hos_version,
    last_load_result,
    loader_info,
    main_thread,
    main_thread_handle,
    own_process_handle,
    random_seed,
    service_overrides,
    set_exit_func_ptr,
    set_next_load,
    syscall_hints,
    user_id_storage,
};
use nx_rt_core::{
    env::init_once,
    services::sm,
};
use nx_svc::thread::Handle as ThreadHandle;

/// Atmosphere flag bit, OR-ed into the HOS version when Atmosphere is detected.
const HOS_VERSION_ATMOSPHERE_BIT: u32 = 1 << 31;

/// Parses the homebrew loader configuration and populates the runtime
/// environment state.
///
/// Runs exactly once; repeat calls are no-ops. Loader-supplied service
/// overrides are registered with the Service Manager as they are parsed.
///
/// A launch with no configuration block is malformed, and the block is the
/// only thing missing: the main-thread handle and the return address are
/// arguments in their own right, so they are recorded either way and the rest
/// of startup runs on the defaults.
///
/// Parsing is all this does. Publishing the parsed applet type to the C-facing
/// global is the caller's step, because that global belongs to the C boundary
/// and this module sits below it.
///
/// # Safety
///
/// `ctx`, when present, must point to a valid `ConfigEntry` array terminated
/// by an `EndOfList` entry, and `main_thread` must be the handle the loader
/// supplied for the process main thread.
pub unsafe fn setup(
    ctx: Option<NonNull<ConfigEntry>>,
    main_thread: ThreadHandle,
    saved_lr: LoaderReturnFn,
) {
    init_once(main_thread, |state| {
        // A homebrew NRO is unconditionally a non-NSO process.
        state.is_nso = false;

        set_exit_func_ptr(saved_lr);

        let Some(ctx) = ctx else {
            return;
        };

        // SAFETY: the caller guarantees `ctx` points to a valid ConfigEntry
        // array terminated by EndOfList.
        let entries = unsafe { ConfigEntries::from_ptr(ctx) };

        for entry in entries {
            match entry {
                Entry::HosVersion {
                    version,
                    is_atmosphere,
                } => {
                    let mut v = version;
                    if is_atmosphere {
                        v |= HOS_VERSION_ATMOSPHERE_BIT;
                    }
                    hos_version::set(v);
                }
                Entry::MainThreadHandle(handle) => {
                    state.main_thread_handle = Some(handle);
                }
                Entry::ProcessHandle(handle) => {
                    state.process_handle = Some(handle);
                }
                Entry::OverrideHeap(heap) => {
                    state.heap_override = Some((heap.cast(), heap.len()));
                }
                Entry::Argv(argv) => {
                    // The C-facing accessors below hand out the bare pointer,
                    // so the borrow is unwrapped back to one here rather than
                    // held.
                    state.argv = Some(NonNull::from(argv).cast());
                }
                Entry::RandomSeed(seed) => {
                    state.random_seed = Some(seed);
                }
                Entry::SyscallHint {
                    hint_0_3f,
                    hint_40_7f,
                } => {
                    state.syscall_hints.set_hints_0_3f(hint_0_3f);
                    state.syscall_hints.set_hints_40_7f(hint_40_7f);
                }
                Entry::SyscallHint2 { hint_80_bf } => {
                    state.syscall_hints.set_hints_80_bf(hint_80_bf);
                }
                Entry::UserIdStorage(storage) => {
                    // The handover sizes the storage in bytes and this layer
                    // names it an `AccountUid`; this is the one place the two
                    // are known to be the same thing.
                    state.user_id_storage = Some(storage.cast());
                }
                Entry::LastLoadResult(result) => {
                    state.last_load_result = result;
                }
                Entry::NextLoadPath { path, argv } => {
                    // SAFETY: The pointers come straight from the loader's
                    // configuration, which is where `from_loader` expects them
                    // from.
                    state.next_load = Some(unsafe { NextLoad::from_loader(path, argv) });
                }
                Entry::OverrideService {
                    name,
                    handle: service_handle,
                } => {
                    if state.service_override_count < MAX_SERVICE_OVERRIDES {
                        state.service_overrides[state.service_override_count] =
                            Some(ServiceOverride::new(name, service_handle));
                        state.service_override_count += 1;
                    }
                    // Register the override with the Service Manager so
                    // lookups resolve to the loader-supplied session. A full
                    // table drops this one, and that name then resolves
                    // through the Service Manager as it would for a process
                    // the loader handed no override at all.
                    let _ = sm::add_override(name, service_handle);
                }
                Entry::AppletType { kind, flags } => {
                    state.applet_type = AppletType::from_raw(kind, flags);
                }
                Entry::AppletWorkaround => {
                    state.applet_workaround = true;
                }
                Entry::LoaderInfo(text) => {
                    // Kept as a pointer and a length because the C-facing
                    // accessors hand the two out separately.
                    state.loader_info =
                        text.map(|text| (NonNull::from(text).cast(), text.len() as u64));
                }
                // TODO: return to the loader when `flags.is_mandatory()`, which
                //  the handover requires and startup does not yet do. Skipping
                //  one leaves the program running against an environment the
                //  loader said it must adopt.
                Entry::Unknown { .. } => {
                    // A key from a newer loader. Skipping it is what the
                    // handover asks for, so long as it is not mandatory.
                }
                // TODO: return to the loader when `flags.is_mandatory()`, on
                //  the same terms as an unknown key above. A loader that named
                //  the heap and gave a null address is one that is not working,
                //  and continuing runs the program on the default heap the
                //  loader was replacing.
                Entry::Malformed { .. } => {
                    // A key this runtime knows, carrying nothing it can act on.
                    // Startup continues on the defaults, which is what it would
                    // have done had the loader not sent the entry at all.
                }
            }
        }
    });
}
