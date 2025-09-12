use std::{
    io::Result,
    time::{Duration, Instant},
};

use crossterm::event::{Event, KeyEvent, KeyEventKind, poll, read};

pub struct UIEvents {
    tick_rate: Duration,
}

pub struct Cfg {
    pub tick_rate: Duration,
}

impl Default for Cfg {
    fn default() -> Cfg {
        Cfg {
            tick_rate: Duration::from_millis(250),
        }
    }
}

impl UIEvents {
    pub fn new() -> UIEvents {
        UIEvents::with_config(Cfg::default())
    }

    pub fn with_config(config: Cfg) -> UIEvents {
        UIEvents {
            tick_rate: config.tick_rate,
        }
    }

    /// Read the next key press event, dropping any other preceding events. Returns None if no
    /// relevant event is found within the configured tick_rate.
    pub fn next_key_press(&self) -> Result<Option<KeyEvent>> {
        let deadline = Instant::now() + self.tick_rate;
        loop {
            let timeout = deadline.saturating_duration_since(Instant::now());
            if !poll(timeout)? {
                return Ok(None);
            }
            if let Event::Key(event) = read()?
                && event.kind == KeyEventKind::Press
            {
                return Ok(Some(event));
            }
        }
    }

    /// Read the next key press event, dropping any other preceding events. If no key event is
    /// available, returns immediately.
    pub fn try_next_key_press(&self) -> Result<Option<KeyEvent>> {
        loop {
            if !poll(Duration::ZERO)? {
                return Ok(None);
            }
            if let Event::Key(event) = read()?
                && event.kind == KeyEventKind::Press
            {
                return Ok(Some(event));
            }
        }
    }
}
