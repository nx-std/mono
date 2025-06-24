//! FFI bindings for the `nx-sys-thread` crate
//!
//! # References
//! - [switchbrew/libnx: switch/runtime/newlib.c](https://github.com/switchbrew/libnx/blob/master/nx/source/runtime/newlib.c)

mod newlib {
    use core::{
        ffi::{c_int, c_long, c_uint},
        ptr,
    };

    use nx_svc::thread as svc;

    // Error codes
    const EFAULT: c_int = 14;
    const EINVAL: c_int = 22;

    #[repr(C)]
    #[derive(Default, Copy, Clone)]
    struct TimeSpec {
        tv_sec: c_long,
        tv_nsec: c_long,
    }

    /// Overrides the `sleep` function from the C standard library.
    ///
    /// This function is declared in `<unistd.h>`.
    #[unsafe(no_mangle)]
    unsafe extern "C" fn __nx_std_thread_newlib_sleep(seconds: c_uint) -> c_uint {
        let nanos = (seconds as u64) * 1_000_000_000;
        let _ = svc::sleep(nanos);
        0
    }

    /// Overrides the `usleep` function from the C standard library.
    ///
    /// This function is declared in `<unistd.h>`.
    #[unsafe(no_mangle)]
    unsafe extern "C" fn __nx_std_thread_newlib_usleep(useconds: c_uint) -> c_int {
        let nanos = (useconds as u64) * 1_000;
        let _ = svc::sleep(nanos);
        0
    }

    /// Overrides the `nanosleep` function from the C standard library.
    ///
    /// This function is declared in `<time.h>`.
    #[unsafe(no_mangle)]
    unsafe extern "C" fn __nx_std_thread_newlib_nanosleep(
        req: *const TimeSpec,
        rem: *mut TimeSpec,
    ) -> c_int {
        if req.is_null() {
            set_errno(EFAULT);
            return -1;
        }

        let request = unsafe { &*req };
        if request.tv_sec < 0 || request.tv_nsec < 0 || request.tv_nsec >= 1_000_000_000 {
            set_errno(EINVAL);
            return -1;
        }

        let nanos = (request.tv_sec as u64) * 1_000_000_000 + (request.tv_nsec as u64);
        let _ = svc::sleep(nanos);

        if !rem.is_null() {
            unsafe { ptr::write(rem, TimeSpec::default()) };
        }

        0
    }

    /// Overrides the `sched_yield` function from the C standard library.
    ///
    /// This function is declared in `<sched.h>`.
    #[unsafe(no_mangle)]
    unsafe extern "C" fn __nx_std_thread_newlib_sched_yield() -> c_int {
        svc::yield_with_migration();
        0
    }

    /// Sets the thread-local `errno` value
    #[inline]
    fn set_errno(code: c_int) {
        unsafe extern "C" {
            // This is a newlib/libc function
            fn __errno() -> *mut c_int;
        }

        unsafe { *__errno() = code };
    }
}
