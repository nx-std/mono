//! # Runtime Environment State (NRO)
//!
//! Parses the homebrew loader's ABI configuration block and populates the
//! kind-agnostic [runtime environment state][nx_rt_core::env] every other
//! consumer reads back.
//!
//! The homebrew loader hands an NRO a `ConfigEntry` array describing the heap
//! override, command-line arguments, service overrides, applet type, HOS
//! version, and the rest of the startup environment. [`setup`] walks that
//! array exactly once and fills `nx-rt-core`'s environment container; the
//! loader-config parser itself lives in the private [`config`] submodule.
//!
//! The read accessors and the HOS-version / main-thread submodules are
//! re-exported from [`nx_rt_core::env`] so callers continue to reach them
//! through `crate::env::*`.

mod config;

use core::ptr::NonNull;

pub use nx_rt_core::env::{
    AccountUid, AppletType, LoaderReturnFn, MAX_SERVICE_OVERRIDES, ServiceName, ServiceOverride,
    SyscallHints, applet_type, applet_workaround, argv, exit_func_ptr, has_next_load,
    heap_override, hos_version, is_nso, last_load_result, loader_info, main_thread,
    main_thread_handle, own_process_handle, random_seed, service_overrides, set_exit_func_ptr,
    set_next_load, syscall_hints, user_id_storage,
};
use nx_rt_core::{env::init_once, services::sm};
use nx_svc::{
    ipc::Handle as ServiceHandle, process::Handle as ProcessHandle, thread::Handle as ThreadHandle,
};

pub use self::config::{ConfigEntries, ConfigEntry, Entry};

/// Atmosphere flag bit, OR-ed into the HOS version when Atmosphere is detected.
const HOS_VERSION_ATMOSPHERE_BIT: u32 = 1 << 31;

/// Parses the homebrew loader configuration and populates the runtime
/// environment state.
///
/// Runs exactly once — repeat calls are no-ops. A homebrew NRO is always a
/// non-NSO process launched with a loader-supplied configuration block, so
/// there is no NSO branch: `ctx` is non-null by construction. Loader-supplied
/// service overrides are registered with the Service Manager as they are
/// parsed.
///
/// # Safety
///
/// `ctx` must point to a valid `ConfigEntry` array terminated by an
/// `EndOfList` entry, and `main_thread` must be the handle the loader supplied
/// for the process main thread.
pub unsafe fn setup(
    ctx: NonNull<ConfigEntry>,
    main_thread: ThreadHandle,
    saved_lr: LoaderReturnFn,
) {
    init_once(|state| {
        // A homebrew NRO is unconditionally a non-NSO process.
        state.is_nso = false;
        // Seed the main-thread handle from the kernel-supplied argument; the
        // loader config's MainThreadHandle entry, when present, confirms it.
        state.main_thread_handle = Some(main_thread);

        set_exit_func_ptr(saved_lr);

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
                Entry::MainThreadHandle(raw) => {
                    // SAFETY: The handle is provided by the loader and guaranteed valid.
                    state.main_thread_handle = Some(ThreadHandle::from_raw_unchecked(raw));
                }
                Entry::ProcessHandle(raw) => {
                    // SAFETY: The handle is provided by the loader and guaranteed valid.
                    state.process_handle = Some(ProcessHandle::from_raw_unchecked(raw));
                }
                Entry::OverrideHeap { addr, size } => {
                    state.heap_override = addr.map(|a| (a, size));
                }
                Entry::Argv(ptr) => {
                    state.argv = ptr;
                }
                Entry::RandomSeed(seed) => {
                    state.random_seed = Some(seed);
                }
                Entry::SyscallHint {
                    hint_0_3f,
                    hint_40_7f,
                } => {
                    state
                        .syscall_hints
                        .get_or_insert_with(SyscallHints::new)
                        .set_hint_0_7f(hint_0_3f, hint_40_7f);
                }
                Entry::SyscallHint2 { hint_80_bf } => {
                    state
                        .syscall_hints
                        .get_or_insert_with(SyscallHints::new)
                        .set_hint_80_bf(hint_80_bf);
                }
                Entry::UserIdStorage(ptr) => {
                    state.user_id_storage = ptr;
                }
                Entry::LastLoadResult(result) => {
                    state.last_load_result = result;
                }
                Entry::NextLoadPath => {
                    state.has_next_load = true;
                }
                Entry::OverrideService { name, handle } => {
                    // SAFETY: The handle is provided by the loader and guaranteed valid.
                    let service_handle = ServiceHandle::from_raw_unchecked(handle);
                    if state.service_override_count < MAX_SERVICE_OVERRIDES {
                        state.service_overrides[state.service_override_count] =
                            Some(ServiceOverride::new(name, service_handle));
                        state.service_override_count += 1;
                    }
                    // Register the override with the Service Manager so SM
                    // lookups resolve to the loader-supplied session.
                    let _ = sm::add_override(name, service_handle);
                }
                Entry::AppletType { kind, flags } => {
                    state.applet_type = AppletType::from_raw(kind, flags);
                }
                Entry::AppletWorkaround => {
                    state.applet_workaround = true;
                }
                Entry::LoaderInfo { ptr, len } => {
                    if len > 0 {
                        state.loader_info = ptr.map(|p| (p, len));
                    }
                }
                Entry::Unknown { .. } => {
                    // Ignore unknown entry types.
                }
            }
        }
    });
}
