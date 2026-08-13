//! The installed process command line, and the scanner that produced it.
//!
//! Why the store sits in this crate rather than in the entry crate that reads
//! the command line is the crate-level documentation's subject.
//!
//! The buffer is reached through [`CommandLine`], which holds the loader's
//! bytes and the spans cut from them together. Keeping the two in one place is
//! what lets a span be trusted: nothing constructs one except the scan that
//! measured it against the buffer beside it, so reading an argument back is
//! ordinary slice indexing rather than something a caller has to be careful
//! about.

use core::cell::UnsafeCell;

use nx_sys_sync::Once;

/// Most arguments a command line can carry.
///
/// Arguments past this are dropped: the store is a fixed array, because a
/// growable one would need an allocator and this crate is reachable before one
/// exists. A Switch process is launched by a homebrew loader or by the process
/// manager, neither of which composes a command line anywhere near this long.
pub const MAX_ARGS: usize = 32;

/// The process's command line, and the guard that orders the one write to it.
///
/// The guard and what it guards are one static rather than two, because a program that links this
/// crate twice must share both or neither: a borrowed command line behind a private guard is a slot
/// two libraries would install into without ordering. Keeping them together makes that
/// unrepresentable.
///
/// The symbol is spelled out, and `extern-state` swaps this definition for a declaration. See
/// [rust-process-wide-state](../../../docs/code/rust-process-wide-state.md).
#[cfg(not(feature = "extern-state"))]
#[unsafe(no_mangle)]
static COMMAND_LINE: InstalledCommandLine = InstalledCommandLine {
    init: Once::new(),
    slot: UnsafeCell::new(CommandLine::EMPTY),
};

#[cfg(feature = "extern-state")]
unsafe extern "Rust" {
    /// The command line and its guard, owned by another static library.
    static COMMAND_LINE: InstalledCommandLine;
}

/// The one command-line slot, however this build reaches it.
fn command_line() -> &'static InstalledCommandLine {
    #[cfg(not(feature = "extern-state"))]
    {
        &COMMAND_LINE
    }

    #[cfg(feature = "extern-state")]
    // SAFETY: the symbol is defined by the one static library built without `extern-state`, as an
    // `InstalledCommandLine` from this same source at this same version, so the reference has the
    // type and layout it claims. It is a `static`, so the `'static` lifetime is honest. The `Once`
    // inside orders the write to the slot; a shared reference to the pair races with nothing.
    unsafe {
        &COMMAND_LINE
    }
}

/// Returns an iterator over the command-line arguments, as `std::env::args_os`
/// does.
///
/// The first argument is typically the program name. Each argument is the bytes
/// the loader delivered, unvalidated and not copied. The iterator is empty until
/// an entry crate has called [`setup_from`], and when the process was launched
/// with no command line at all.
pub fn args() -> Args {
    Default::default()
}

/// Installs the process command line from an already-read argument string.
///
/// `source` is the raw, whitespace- and quote-delimited argument string the
/// entry crate's kind-specific reader produced, **including its terminating
/// nul**: the last byte is what terminates the final argument once the
/// separators around it have been overwritten, so it has to be a byte this
/// slice covers rather than one just past its end.
///
/// The slice must stop there. A loader's buffer is often larger than the string
/// in it, and the bytes past the terminator are capacity that was never
/// written, which is not memory to read whatever its address is mapped to.
///
/// Scanning writes a nul after each argument, in the separator byte that
/// already followed it, so that both [`args`] and the C-facing view read the
/// loader's own buffer rather than a copy of it. Nothing is allocated.
///
/// Runs exactly once; subsequent calls are no-ops, which is what keeps the
/// unsynchronized reads in [`Args`] sound. Arguments past [`MAX_ARGS`] are
/// dropped.
pub fn setup_from(source: &'static mut [u8]) {
    let command_line = command_line();

    command_line.init.call_once(|| {
        let scanned = CommandLine::scan(source);

        // SAFETY: `Once` runs this at most once and blocks every other caller
        // until it returns, so this is the only writer, and no reader can be
        // running: `installed` reads only after `is_completed` observes this
        // call finished.
        unsafe { *command_line.slot.get() = scanned };
    });
}

/// Iterator over the command-line arguments, as `std::env::ArgsOs` is.
///
/// Arguments are borrowed rather than copied: they are subslices of a buffer
/// that outlives the program, so there is nothing for a caller to own.
#[derive(Default)]
pub struct Args {
    index: usize,
}

impl Iterator for Args {
    type Item = &'static [u8];

    fn next(&mut self) -> Option<&'static [u8]> {
        let arg = installed()?.arg(self.index)?;

        self.index += 1;
        Some(arg)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = installed().map_or(0, |command_line| {
            command_line.count().saturating_sub(self.index)
        });
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for Args {}

/// Borrows the installed command line, or `None` before one is installed.
fn installed() -> Option<&'static CommandLine> {
    let command_line = command_line();

    if !command_line.init.is_completed() {
        return None;
    }

    // SAFETY: `is_completed` is true, so the one write in `setup_from` has
    // finished and nothing writes again. The slot lives in a `static`, so the
    // borrow is good for the rest of the program.
    Some(unsafe { &*command_line.slot.get() })
}

/// A scanned command line: the loader's buffer, and where each argument sits
/// inside it.
///
/// The two travel together so that a [`Span`] never has to be checked against a
/// buffer it might not belong to. [`scan`](Self::scan) is the only constructor
/// that produces spans, and it measures each one against the same `source` it
/// stores, so [`arg`](Self::arg) is plain indexing.
struct CommandLine {
    /// The argument string, nul-separated by [`scan`](Self::scan).
    source: &'static [u8],
    /// Where each argument sits within `source`.
    spans: [Span; MAX_ARGS],
    /// How many entries of `spans` are live.
    count: usize,
}

impl CommandLine {
    /// The command line of a process that was launched without one.
    const EMPTY: Self = Self {
        source: &[],
        spans: [Span { start: 0, len: 0 }; MAX_ARGS],
        count: 0,
    };

    /// Splits `source` in place, recording where each argument sits.
    ///
    /// Arguments are separated by ASCII whitespace; a `"` pair quotes one whole
    /// argument, so that an argument is always a contiguous run of the buffer
    /// and can be named by a span rather than copied out. The scan walks bytes
    /// rather than characters because every byte of a multi-byte UTF-8 sequence
    /// is `0x80` or above, so neither delimiter can be mistaken for a fragment
    /// of one.
    ///
    /// Each argument is nul-terminated where it ends, overwriting the separator
    /// that closed it. The last byte of `source` is the terminator the caller
    /// supplied, which is where the final argument's nul lands when no
    /// separator follows it.
    fn scan(source: &'static mut [u8]) -> Self {
        // Every byte but the last is argument text; the last is the terminator
        // slot. A buffer with no room for one carries no arguments either.
        let Some(content_len) = source.len().checked_sub(1) else {
            return Self::EMPTY;
        };

        let mut spans = [Span { start: 0, len: 0 }; MAX_ARGS];
        let mut count = 0;
        let mut index = 0;

        while index < content_len && count < MAX_ARGS {
            if source[index].is_ascii_whitespace() {
                index += 1;
                continue;
            }

            // A quote opens an argument that runs to the next quote; anything
            // else opens one that runs to the next whitespace.
            let quoted = source[index] == b'"';
            let start = if quoted { index + 1 } else { index };
            let mut end = start;
            while end < content_len {
                let closes = if quoted {
                    source[end] == b'"'
                } else {
                    source[end].is_ascii_whitespace()
                };
                if closes {
                    break;
                }
                end += 1;
            }

            if end > start {
                spans[count] = Span {
                    start,
                    len: end - start,
                };
                count += 1;
            }

            // Terminate the argument in place. `end` is the separator that
            // closed it, or `content_len`, which is the caller's terminator.
            source[end] = 0;
            index = end + 1;
        }

        Self {
            source,
            spans,
            count,
        }
    }

    /// How many arguments the command line carries.
    fn count(&self) -> usize {
        self.count
    }

    /// Borrows argument `index`, or `None` past the last one.
    fn arg(&self, index: usize) -> Option<&'static [u8]> {
        if index >= self.count {
            return None;
        }
        let span = self.spans[index];

        // `source` is `&'static`, so a subslice of it is too. `scan` measured
        // every live span against this same buffer, so the range is in bounds
        // and the indexing below cannot panic.
        Some(&self.source[span.start..span.start + span.len])
    }
}

/// Where one argument sits within the argument string.
///
/// Only [`CommandLine::scan`] builds one, always against the buffer stored
/// beside it, which is what makes [`CommandLine::arg`] safe to index with.
#[derive(Clone, Copy)]
struct Span {
    /// Byte offset of the argument's first byte.
    start: usize,
    /// Length in bytes, excluding the nul that terminates it.
    len: usize,
}

/// The write-once command-line store: the slot, and the guard that orders it.
struct InstalledCommandLine {
    /// Ensures the slot is written once, and answers whether it has been.
    init: Once,
    /// The command line, empty until [`setup_from`] installs one.
    slot: UnsafeCell<CommandLine>,
}

// SAFETY: the slot is written once inside `init`, which blocks every other
// caller until that write returns, and read only once `is_completed` observes
// it. No reader can see it mid-write, and nothing writes it twice.
unsafe impl Sync for InstalledCommandLine {}
