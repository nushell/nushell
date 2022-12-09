use std::{
    io::Result,
    time::{Duration, Instant},
};

use crossterm::event::{poll, read, Event, KeyEvent};

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

    pub fn next(&self) -> Result<Option<KeyEvent>> {
        let now = Instant::now();
        match poll(self.tick_rate) {
            Ok(true) => match read()? {
                Event::Key(event) => Ok(Some(event)),
                _ => {
                    let time_spent = now.elapsed();
                    let rest = self.tick_rate - time_spent;

                    Self { tick_rate: rest }.next()
                }
            },
            Ok(false) => Ok(None),
            Err(err) => Err(err),
        }
    }

    pub fn try_next(&self) -> Result<Option<KeyEvent>> {
        match poll(Duration::default()) {
            Ok(true) => match read()? {
                Event::Key(event) => Ok(Some(event)),
                _ => Ok(None),
            },
            Ok(false) => Ok(None),
            Err(err) => Err(err),
        }
    }
}
