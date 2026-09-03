use crate::*;

/// Run completions on a background thread instead of blocking the line editor.
///
/// Disabling restores the pre-0.115 behavior: completions settle on the
/// reedline thread, which blocks while they run, so a custom completer can
/// hand the terminal to an interactive picker (`$candidates | fzf`) without
/// the line editor stealing its keystrokes.
pub static BACKGROUND_COMPLETIONS: ExperimentalOption =
    ExperimentalOption::new(&BackgroundCompletions);

// No documentation needed here since this type isn't public.
// The static above provides all necessary details.
struct BackgroundCompletions;

impl ExperimentalOptionMarker for BackgroundCompletions {
    const IDENTIFIER: &'static str = "background-completions";
    const DESCRIPTION: &'static str = "\
        Runs completions on a background thread so typing never blocks. \
        Disable to let custom completers drive interactive child processes.";
    const STATUS: Status = Status::OptOut;
    const SINCE: Version = (0, 115, 0);
    const ISSUE: u32 = 18839;
}
