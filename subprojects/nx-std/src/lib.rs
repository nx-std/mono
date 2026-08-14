//! # nx-std
#![no_std]

extern crate nx_panic_handler; // provides #[panic_handler]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "alloc")]
pub extern crate nx_alloc; // Provides #[global_allocator]

// FFI exports - re-export FFI symbols from dependent crates to ensure they're
// included in the staticlib. This module is only compiled when the `ffi` feature
// is enabled.
#[cfg(feature = "ffi")]
pub mod ffi;

#[cfg(feature = "env")]
pub mod env;
#[cfg(feature = "fs")]
pub mod fs;
#[cfg(feature = "thread")]
pub mod thread;
#[cfg(feature = "sync")]
pub mod sync {
    pub use nx_std_sync::*;
}
#[cfg(feature = "time")]
pub mod time {
    pub use nx_time::*;
}
// One entry crate per launch path, and a link takes exactly one of them: the
// homebrew-loader runtime or the `pm`-launched process runtime. The Meson
// `nx_rt_kind` combo picks which, so only its module is compiled in.
#[cfg(feature = "rt-hbapp")]
pub mod rt_hbapp {
    pub use nx_rt_hbapp::*;
}
#[cfg(feature = "rt-nso")]
pub mod rt_nso {
    pub use nx_rt_nso::*;
}
#[cfg(feature = "display")]
pub mod display {
    pub use nx_display::*;
}

#[cfg(feature = "services")]
pub mod services {
    #[cfg(feature = "service-apm")]
    pub mod apm {
        pub use nx_service_apm::*;
    }
    #[cfg(feature = "service-applet")]
    pub mod applet {
        pub use nx_service_applet::*;
    }
    #[cfg(feature = "service-hid")]
    pub mod hid {
        pub use nx_service_hid::*;
    }
    #[cfg(feature = "service-set")]
    pub mod set {
        pub use nx_service_set::*;
    }
    #[cfg(feature = "service-sfdnsres")]
    pub mod sfdnsres {
        pub use nx_service_sfdnsres::*;
    }
    #[cfg(feature = "service-sm")]
    pub mod sm {
        pub use nx_service_sm::*;
    }
    #[cfg(feature = "service-time")]
    pub mod time {
        pub use nx_service_time::*;
    }
    #[cfg(feature = "service-vi")]
    pub mod vi {
        pub use nx_service_vi::*;
    }
}

#[cfg(any(
    feature = "sys",
    feature = "alloc",
    feature = "svc",
    feature = "sys-mem",
    feature = "sys-sync",
    feature = "sys-thread",
    feature = "sys-thread-tls"
))]
pub mod sys {
    #[cfg(any(feature = "sys", feature = "alloc"))]
    pub use nx_alloc as alloc;
    #[cfg(any(feature = "sys", feature = "svc"))]
    pub use nx_svc as svc;
    #[cfg(any(feature = "sys", feature = "sys-mem"))]
    pub use nx_sys_mem as mem;
    #[cfg(any(feature = "sys", feature = "sys-sync"))]
    pub use nx_sys_sync as sync;
    #[cfg(any(feature = "sys", feature = "sys-thread"))]
    pub use nx_sys_thread as thread;
    #[cfg(any(feature = "sys", feature = "sys-thread-tls"))]
    pub use nx_sys_thread_tls as thread_tls;
}
