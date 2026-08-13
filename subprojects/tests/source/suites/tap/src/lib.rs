//! # nx-tests-tap
//!
//! Reporting in the Test Anything Protocol, version 14, for the suites under `subprojects/tests`.
//!
//! A run that nobody watches has to be readable by something that is not a person. TAP is the
//! format that something already reads: it is line-oriented, it survives being streamed, and every
//! language has a harness that consumes it. See <https://testanything.org/>.
//!
//! ## Three readers that cannot reach each other
//!
//! A suite reports the same document three ways, because no one of them reaches everybody:
//!
//! - **The console**, as each case finishes, for whoever is standing in front of it. It is written
//!   to as the run happens because it is the only destination with somebody watching, and it keeps
//!   nothing: libnx draws each character to the framebuffer and holds no buffer, so a line that was
//!   only shown cannot be read back afterwards.
//! - **The card**, once the cases are over. This is what makes a run launched by hand readable at
//!   all, and it is written whether or not anyone is listening elsewhere.
//! - **The host**, once the cases are over. The only reader a run can be *driven* from, and the
//!   only one that can act on what it reads.
//!
//! ## The document is accumulated, not recovered
//!
//! The card and the host need the whole run at once, and the console was told it a line at a time.
//! So the document keeps every case as it is reported and renders all three from that. The
//! alternative -- recovering the run at the end from the fixed table the harness records into for a
//! debugger to read -- is what this replaces: that table has a capacity, and a document rebuilt
//! from it stops short exactly when a run is long enough to be worth reading.
//!
//! ## What the caller owns, and why
//!
//! Two things this deliberately does not reach for, because reaching for them from here would take
//! them away from the program that already has them:
//!
//! - **The socket driver.** Bringing one up needs the service-manager session the runtime owns, and
//!   a suite that brought one up for its own cases would have it taken down underneath it. So the
//!   caller guarantees a driver is running before it asks for a report, and [`report`] connects
//!   over it. See [`host`].
//! - **The system version and the host's address.** Both are facts the C runtime holds and nothing
//!   below it may ask for. They arrive in [`Run`] and in [`report`]'s argument, from the `main` that
//!   already has them.
//!
//! ## One archive per binary
//!
//! This crate holds process-wide state -- the open document -- and takes [`nx_sys_net`] and
//! [`nx_std_fs`], which hold their own. A `static` is only process-wide while the crate holding it
//! is linked once, so a test binary links **exactly one** Rust static library and everything else
//! reaches it as an rlib inside that one.
//!
//! While the cases are still C, that archive is this crate, built with `ffi`; its dependencies take
//! `extern-state` so the driver and the descriptor table it borrows are the ones the program
//! already brought up. As a suite's cases move to Rust the suite's own crate becomes the archive
//! and this becomes an rlib inside it, at which point `ffi` moves with it. See
//! `docs/code/rust-process-wide-state.md`.
//!
//! ## no-std
//!
//! The crate is `#![no_std]` and uses `alloc` for the document it accumulates; the archive it is
//! linked into carries the single `#[global_allocator]`.
#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

// The document is built up over a run, so its size is not known until the run is over.
extern crate alloc;
// `nx-alloc` exposes the `#[global_allocator]` backing `alloc` for this crate.
extern crate nx_alloc as _;

use core::net::Ipv4Addr;

use nx_std_sync::mutex::Mutex;

pub mod card;
pub mod host;

// Neither is named in this crate's API: the console is written to on the way past, and the
// document is what the entry points below keep between them.
mod console;
mod document;

#[cfg(feature = "ffi")]
pub mod ffi;

pub use self::document::{
    HosVersion,
    Outcome,
    Run,
};

/// The run in progress, or `None` before [`begin`] opens one and after nothing more can be added.
///
/// One per process, because a program runs one suite: the binary is the unit the runner hands over,
/// and what it reports is the whole of what it did.
///
/// The lock is held for the length of one operation. Cases run on threads of their own, but they
/// are reported from the thread that ran them to completion, one at a time, so it is never
/// contended in practice -- it is here because a `static` that is written to has to be.
static DOCUMENT: Mutex<Option<document::Document>> = Mutex::new(None);

/// Opens the document. Called once, before any case runs.
///
/// Anything reported before this is dropped: there is nowhere to put it, and a document that
/// silently began itself would be one whose preamble said nothing about the run.
pub fn begin(run: Run) {
    let mut guard = DOCUMENT.lock();
    let document = guard.insert(document::Document::new(run));

    console::write(document::VERSION_LINE);
    console::write(&document.preamble());
}

/// Writes a line the protocol carries and ignores, for whatever a reader may want.
///
/// Goes to the console alone. A comment is context for somebody watching a run happen; the document
/// the card and the host are sent is the cases.
pub fn comment(text: &str) {
    console::write(&document::render_comment(text));
}

/// Reports one case, numbering it after the last one reported.
pub fn case(title: &str, outcome: Outcome) {
    let mut guard = DOCUMENT.lock();
    let Some(document) = guard.as_mut() else {
        return;
    };

    let line = document.push(title, outcome);
    console::write(&line);
}

/// Closes the document by stating how many cases there were.
pub fn plan() {
    let guard = DOCUMENT.lock();
    let Some(document) = guard.as_ref() else {
        return;
    };

    console::write(&document.plan());
}

/// Files the document with the card and sends it to the host.
///
/// Called once the cases are over. Nothing here runs while they do: the connection this opens would
/// otherwise be sharing the process with the threads and timings under test.
///
/// `host` is the address the program was pushed from, which a suite launched by hand does not have.
/// A caller that passes one guarantees a socket driver is already running, since bringing one up is
/// the program's to do and not this crate's.
///
/// Returns what became of each destination. Neither failing is a failure of the run, and a caller
/// that has nowhere to put the answer can drop it: the console has already shown every case.
pub fn report(host: Option<Ipv4Addr>) -> Report {
    let guard = DOCUMENT.lock();
    let Some(document) = guard.as_ref() else {
        return Report {
            card: Ok(()),
            host: Ok(()),
        };
    };

    let text = document.render();

    let card = card::write(document.report_dir(), document.suite(), &text);

    let host = match host {
        None => Ok(()),
        Some(addr) => host::Host::connect(addr).map(|host| {
            host.write(&text);
            host.close();
        }),
    };

    Report { card, host }
}

/// What became of each destination [`report`] wrote to.
pub struct Report {
    /// Whether the card kept the document.
    pub card: Result<(), card::WriteError>,
    /// Whether the host was reached, and `Ok(())` when there was no host to reach.
    pub host: Result<(), host::ConnectError>,
}
