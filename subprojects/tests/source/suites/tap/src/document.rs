//! The document itself: what a run is, what became of each case, and the text either turns into.
//!
//! The one place that knows what the protocol looks like. All three destinations go through it, so
//! the console sees the same line as the card and the host rather than a second rendering that
//! drifted from the first.

use alloc::{
    string::{
        String,
        ToString as _,
    },
    vec::Vec,
};
use core::fmt::Write as _;

/// The version line every document opens with.
pub const VERSION_LINE: &str = "TAP version 14\n";

/// What a reader needs in order to know which run it is looking at.
///
/// The same suite reports differently depending on how the build was configured and who launched
/// it, so none of this can be inferred from the case list.
#[derive(Debug, Clone)]
pub struct Run {
    /// The name this suite reports under, which is also the name its file is written to.
    pub suite: String,
    /// The build this was compiled from.
    pub build: String,
    /// The directory the report is filed in.
    pub report_dir: String,
    /// The system version the run happened on.
    pub hos: HosVersion,
    /// Whether the runner launched it rather than a person.
    pub unattended: bool,
}

/// The system version a run happened on.
#[derive(Debug, Clone, Copy)]
pub struct HosVersion {
    /// Major, minor and micro, as the system reports them.
    pub major: u8,
    /// See [`HosVersion::major`].
    pub minor: u8,
    /// See [`HosVersion::major`].
    pub micro: u8,
    /// Whether the run happened under a custom firmware.
    ///
    /// Worth saying because it is the difference between two runs that otherwise report the same
    /// version and behave differently.
    pub atmosphere: bool,
}

/// What became of one case.
///
/// An enum rather than the code a case returned, because the protocol treats these six differently
/// and a caller that has already decided which one it is should not be able to say so ambiguously.
#[derive(Debug, Clone)]
pub enum Outcome {
    /// The case ran and everything it asserted held.
    Passed,
    /// The case declined to run, because the console is not in a state it can test.
    ///
    /// A pass that says it did not run: what is missing is a property of the console rather than of
    /// the code under test.
    Skipped,
    /// The case is not written yet.
    ///
    /// A failure the protocol expects, so a reader does not count it against the run.
    Todo,
    /// The fixture could not be built, so the case never ran.
    ///
    /// Distinct from a failure: the thing under test is neither accused nor cleared.
    SetupFailed,
    /// The case ran and something it asserted did not hold.
    Failed {
        /// What the case returned.
        rc: i32,
    },
    /// The machinery around the case failed, so its result is not known.
    HarnessError {
        /// What went wrong, in the harness's own words.
        reason: String,
    },
}

/// One case, as it will be reported.
#[derive(Debug, Clone)]
struct Case {
    /// The title the case was declared with.
    title: String,
    /// What became of it.
    outcome: Outcome,
}

/// A run in progress, accumulating what it will eventually report.
///
/// The console is written to as each case finishes, but the card and the host are written once at
/// the end and need the whole list, so the list is kept here rather than re-read from whatever the
/// console happened to print.
#[derive(Debug)]
pub struct Document {
    /// What this run is.
    run: Run,
    /// Every case reported so far, in the order they were reported.
    cases: Vec<Case>,
}

impl Document {
    /// Opens a document for `run`, with no cases in it yet.
    pub fn new(run: Run) -> Self {
        Self {
            run,
            cases: Vec::new(),
        }
    }

    /// The name this run is filed under.
    pub fn suite(&self) -> &str {
        &self.run.suite
    }

    /// The directory the report belongs in.
    pub fn report_dir(&self) -> &str {
        &self.run.report_dir
    }

    /// Adds a case, and renders the line the console is told about it.
    ///
    /// The two are one step because the number a case reports under is its position in the list,
    /// and letting a caller supply it separately is how the two drift apart.
    pub fn push(&mut self, title: &str, outcome: Outcome) -> String {
        self.cases.push(Case {
            title: title.to_string(),
            outcome,
        });

        let last = self.cases.len() - 1;
        render_case(last + 1, &self.cases[last])
    }

    /// The line stating how many cases there were.
    ///
    /// The count is only known once they have all run, which is why this is the end of the document
    /// rather than the start; the protocol allows either.
    pub fn plan(&self) -> String {
        let mut line = String::new();
        // Writing into a `String` cannot fail.
        let _ = writeln!(&mut line, "1..{}", self.cases.len());
        line
    }

    /// The comments that say which run this is.
    pub fn preamble(&self) -> String {
        let mode = match self.run.unattended {
            true => "unattended",
            false => "interactive",
        };
        let firmware = match self.run.hos.atmosphere {
            true => " (AMS)",
            false => "",
        };

        let mut text = String::new();
        // Writing into a `String` cannot fail.
        let _ = write!(
            &mut text,
            "# suite: {}\n# build: {}\n# hos: {}.{}.{}{}\n# mode: {}\n",
            self.run.suite,
            self.run.build,
            self.run.hos.major,
            self.run.hos.minor,
            self.run.hos.micro,
            firmware,
            mode,
        );
        text
    }

    /// The whole document, from the version line to the plan.
    ///
    /// What the card keeps and what the host is sent. The console was told the same lines as they
    /// happened, so it is not written to again from here.
    pub fn render(&self) -> String {
        let mut text = String::new();
        text.push_str(VERSION_LINE);
        text.push_str(&self.preamble());

        for (index, case) in self.cases.iter().enumerate() {
            text.push_str(&render_case(index + 1, case));
        }

        text.push_str(&self.plan());
        text
    }
}

/// Renders one case's report, newline included.
fn render_case(number: usize, case: &Case) -> String {
    let title = &case.title;

    let mut line = String::new();
    // Writing into a `String` cannot fail, at every arm below.
    let _ = match &case.outcome {
        Outcome::Passed => writeln!(&mut line, "ok {number} - {title}"),
        Outcome::Skipped => writeln!(&mut line, "ok {number} - {title} # SKIP"),
        Outcome::Todo => writeln!(
            &mut line,
            "not ok {number} - {title} # TODO not implemented yet"
        ),
        // Everything below failed, and the indented block under it is where the protocol puts what
        // a reader needs in order to act on the failure.
        Outcome::SetupFailed => writeln!(
            &mut line,
            "not ok {number} - {title}\n  ---\n  reason: the fixture could not be built, so the \
             case never ran\n  ..."
        ),
        Outcome::Failed { rc } => writeln!(
            &mut line,
            "not ok {number} - {title}\n  ---\n  rc: 0x{:08X}\n  ...",
            *rc as u32
        ),
        Outcome::HarnessError { reason } => writeln!(
            &mut line,
            "not ok {number} - {title}\n  ---\n  harness: {reason}\n  ..."
        ),
    };
    line
}

/// Renders a comment, which the protocol carries and ignores.
pub fn render_comment(text: &str) -> String {
    let mut line = String::new();
    // Writing into a `String` cannot fail.
    let _ = writeln!(&mut line, "# {text}");
    line
}
