//! Helper functions for tests that requires a terminal emulator. Terminal
//! emulation is supported by the `alacritty_terminal` crate.
//!
//! Here's the general process of writing a test with a terminal emulator. This
//! module provides helper functions for common cases, but you can always do it
//! on your own. Examples can be found in `tests/terminal`.
//!
//! Step 1: Create an `alacritty_terminal::Term` instance. Here you can
//! configure window size and scrollback buffer size, etc. `default_terminal()`
//! will create one for you with sensible defaults.
//!
//! Step 2: Create a PTY and spawn a Nushell process to the slave end.
//! `pty_with_nushell()` will do that for you. Here you can set PWD or pass
//! command line arguments to Nushell. It's always a good idea to pass
//! `--no-config-file`, otherwise Nushell will ask if you want to create one
//! with default, and that messes up the input.
//!
//! Step 3: Wait for Nushell to initialize (sleeping for 500ms should do). On
//! Linux, trying to write to the PTY before Nushell finishes initialization
//! appears to succeed, but the data will be lost. I'm not sure if this is a bug
//! or just weird behavior of Linux PTY. It's not necessary on Windows, but it
//! won't hurt either.
//!
//! Step 4: Write data to the PTY. Any data you sent will appear to Nushell as
//! if they were typed in a terminal. ANSI escape codes are used for special
//! keystrokes. For example, if you want to press Enter, send "\r" (NOT "\n").
//! On Linux, use `sendkey -a` to see the actual value of a keystroke. The
//! [Wikipedia page](https://en.wikipedia.org/wiki/ANSI_escape_code) also
//! contains a list of common ANSI escape codes.
//!
//! Step 5: Wait for Nushell to respond and update the terminal state. Sometimes
//! Nushell will emit terminal events (e.g. querying cursor position, modifying
//! system clipboard), and these events need to be handled. `read_to_end()` will
//! do that for you, and you can use `pty_write_handler()` for the event handler
//! if you don't care about any of the terminal events.
//!
//! Step 6: Examine the terminal state and make assertions. You can use
//! `extract_text()` and `extract_cursor()` if you only care about the text.

use alacritty_terminal::{
    event::{Event, EventListener, WindowSize},
    grid::Indexed,
    term::{test::TermSize, Config},
    tty::{self, EventedReadWrite, Options, Pty, Shell},
    vte::ansi::{Processor, StdSyncHandler},
    Term,
};
use std::{
    collections::HashMap,
    io::{ErrorKind, Read, Write},
    path::PathBuf,
    sync::mpsc,
    time::Duration,
};

pub struct EventProxy(mpsc::Sender<Event>);

impl EventListener for EventProxy {
    fn send_event(&self, event: Event) {
        let _ = self.0.send(event);
    }
}

/// Creates a 24x80 terminal with default configurations. Returns the terminal
/// and a `mpsc::Receiver` that receives terminal events.
pub fn default_terminal() -> (Term<EventProxy>, mpsc::Receiver<Event>) {
    let config = Config::default();
    let size = TermSize {
        screen_lines: 24,
        columns: 80,
    };
    let (tx, rx) = mpsc::channel();
    (Term::new(config, &size, EventProxy(tx)), rx)
}

/// Creates a PTY and connect the slave end to a Nushell process. If `pwd` is
/// None, the Nushell process will inherit PWD from the current process.
pub fn pty_with_nushell(args: Vec<&str>, pwd: Option<PathBuf>) -> Pty {
    let executable = crate::fs::executable_path().to_string_lossy().to_string();
    let options = Options {
        shell: Some(Shell::new(
            executable,
            args.iter().map(|s| s.to_string()).collect(),
        )),
        working_directory: pwd,
        hold: false,
        env: HashMap::new(),
    };
    let window_size = WindowSize {
        num_lines: 24,
        num_cols: 80,
        cell_width: 0,
        cell_height: 0,
    };
    tty::new(&options, window_size, 0).expect("creating a PTY succeeds")
}

/// Reads from `pty` until no more data is available. Will periodically call
/// `event_handler` to handle terminal events.
pub fn read_to_end<T: EventListener>(
    terminal: &mut Term<T>,
    pty: &mut Pty,
    events: &mut mpsc::Receiver<Event>,
    mut event_handler: impl FnMut(&mut Term<T>, &mut Pty, Event),
) {
    let mut parser: Processor<StdSyncHandler> = Processor::new();
    loop {
        // Read from the PTY.
        let mut buf = [0; 512];
        match pty.reader().read(&mut buf) {
            Ok(n) => {
                if n == 0 {
                    return;
                } else {
                    // Update the terminal state.
                    for byte in &buf[..n] {
                        parser.advance(terminal, *byte);
                    }

                    // Handle terminal events.
                    while let Ok(event) = events.try_recv() {
                        event_handler(terminal, pty, event);
                    }

                    // Poll again after 100ms. The delay is necessary so that
                    // the child process can respond to any new data we might
                    // have sent in the event handler.
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
            Err(err) => {
                if let ErrorKind::Interrupted = err.kind() {
                    // retry
                } else {
                    return;
                }
            }
        }
    }
}

/// An event handler that only responds to `Event::PtyWrite`. This is the
/// minimum amount of event handling you need to get Nushell working.
pub fn pty_write_handler<T: EventListener>(_terminal: &mut Term<T>, pty: &mut Pty, event: Event) {
    if let Event::PtyWrite(text) = event {
        pty.writer()
            .write_all(text.as_bytes())
            .expect("writing to PTY succeeds");
    }
}

/// Extracts the current cursor position.
pub fn extract_cursor<T>(terminal: &Term<T>) -> (usize, usize) {
    let cursor = terminal.grid().cursor.point;
    (cursor.line.0 as usize, cursor.column.0)
}

/// Extracts all visible text, ignoring text styles.
pub fn extract_text<T>(terminal: &Term<T>) -> Vec<String> {
    let mut text: Vec<String> = vec![];
    for Indexed { point, cell } in terminal.grid().display_iter() {
        if point.column == 0 {
            text.push(String::new());
        }
        text.last_mut()
            .expect("terminal grid start at column 0")
            .push(cell.c);
    }
    text
}
