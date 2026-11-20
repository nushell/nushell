//! Asks the terminal for its background color.

use parking_lot::Mutex;
use std::io::IsTerminal;
use terminal_colorsaurus::{Error, QueryOptions, background_color};

/// An sRGB color: red, green and blue.
pub type Rgb = [u8; 3];

/// What is known of the background color of the terminal.
#[derive(Clone, Copy)]
enum Background {
    /// The terminal was never asked.
    Unasked,
    /// The terminal is not to be asked for now: it is at another time.
    Held,
    /// What the terminal answered: `None` when it has no answer, or when there
    /// is no terminal to ask.
    Known(Option<Rgb>),
    /// The terminal answered nothing at all in time: it is not asked again,
    /// since every time would wait that long.
    Silent,
}

static BACKGROUND: Mutex<Background> = Mutex::new(Background::Unasked);

/// The background color of the terminal, or `None` when it is not known: there
/// is no terminal, or it does not tell.
///
/// This is what the terminal last answered. It is asked here only if it never
/// was, which is for programs that run without a line editor. Asking reads the
/// terminal, so a program that has a line editor reading it too must ask at a
/// time of its own with [`update_terminal_background`], before anything calls
/// this.
pub fn terminal_background() -> Option<Rgb> {
    let mut background = BACKGROUND.lock();
    if matches!(*background, Background::Unasked) {
        *background = query_background();
    }
    match *background {
        Background::Known(color) => color,
        Background::Unasked | Background::Held | Background::Silent => None,
    }
}

/// Asks the terminal for its background color if it is `wanted` and not known
/// yet, and forgets it if it is not wanted, so that [`terminal_background`]
/// never asks by itself: a shell calls this with `wanted` false before it runs
/// anything. Not wanting it, then wanting it, is how to have a terminal that
/// changed its colors asked again.
///
/// This is for a shell that is about to read the terminal for a line: it takes
/// the terminal back from an external that still holds it, like one a hook ran
/// last and did not wait for. What was typed and not read yet would be lost to
/// the reading of the reply: `can_ask` tells whether there is no such input,
/// and the terminal is asked at a later call when there is.
pub fn update_terminal_background(wanted: bool, can_ask: impl FnOnce() -> bool) {
    let mut background = BACKGROUND.lock();
    *background = match *background {
        Background::Silent => Background::Silent,
        _ if !wanted => Background::Held,
        known @ Background::Known(_) => known,
        Background::Unasked | Background::Held => {
            take_terminal();
            if can_ask() {
                query_background()
            } else {
                Background::Held
            }
        }
    };
}

/// Asks the terminal with an `OSC 11` query and reads its reply.
///
/// The terminal is asked only when its reply can be read: one nobody reads
/// would show up as typed input.
fn query_background() -> Background {
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() || !in_foreground() {
        return Background::Known(None);
    }
    match background_color(QueryOptions::default()) {
        Ok(color) => Background::Known(Some(color.scale_to_8bit().into())),
        Err(Error::Timeout(_)) => Background::Silent,
        Err(_) => Background::Known(None),
    }
}

/// Whether this process may read the terminal, which only the foreground
/// process group of the terminal may.
fn in_foreground() -> bool {
    #[cfg(unix)]
    {
        use nix::unistd;
        unistd::tcgetpgrp(std::io::stdin()) == Ok(unistd::getpgrp())
    }
    #[cfg(not(unix))]
    true
}

/// Makes this process group the foreground one of the terminal.
fn take_terminal() {
    // A shell that failed to take its terminal is not in the foreground, and
    // so does not ask.
    #[cfg(unix)]
    let _ = nix::unistd::tcsetpgrp(std::io::stdin(), nix::unistd::getpgrp());
}
