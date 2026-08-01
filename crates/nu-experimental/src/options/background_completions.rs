use crate::*;

/// Run completions on a background thread instead of blocking the line editor.
///
/// While the worker computes, reedline keeps polling the terminal for input, so a
/// custom completer that drives an interactive picker (`$candidates | fzf`) paints
/// its UI but never receives any keystrokes; the line editor is reading the same
/// terminal at the same time. Disabling this restores the pre-0.115 behavior: the
/// completer settles on the reedline thread, which blocks while it runs, and
/// expensive completions freeze typing again.
pub static BACKGROUND_COMPLETIONS: ExperimentalOption =
    ExperimentalOption::new(&BackgroundCompletions);

// No documentation needed here since this type isn't public.
// The static above provides all necessary details.
struct BackgroundCompletions;

impl ExperimentalOptionMarker for BackgroundCompletions {
    const IDENTIFIER: &'static str = "background-completions";
    const DESCRIPTION: &'static str = "Runs completions on a background thread so typing never blocks. Disable to let custom completers drive interactive child processes.";
    const STATUS: Status = Status::OptOut;
    const SINCE: Version = (0, 115, 0);
    // TODO: placeholder. Needs a nushell tracking issue recording how this option
    // ends its life, either stabilizing as the default or being removed once
    // interactive completers have a supported path. nushell/reedline#1131 is the
    // discussion, but it lives in the wrong repo to serve as the tracker.
    const ISSUE: u32 = 0;
}
