use chrono::{DateTime, Local};
use core::fmt;
use log::Level;
use log::LevelFilter;
use nu_protocol::ShellError;
use pretty_env_logger::env_logger::fmt::Color;
use pretty_env_logger::env_logger::Builder;
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};

pub fn logger(f: impl FnOnce(&mut Builder) -> Result<(), ShellError>) -> Result<(), ShellError> {
    let mut builder = my_formatted_timed_builder();
    f(&mut builder)?;
    let _ = builder.try_init();
    Ok(())
}

pub fn my_formatted_timed_builder() -> Builder {
    let mut builder = Builder::new();

    builder.format(|f, record| {
        let target = record.target();
        let max_width = max_target_width(target);

        let mut style = f.style();
        let level = colored_level(&mut style, record.level());

        let mut style = f.style();
        let target = style.set_bold(true).value(Padded {
            value: target,
            width: max_width,
        });

        let dt = match DateTime::parse_from_rfc3339(&f.timestamp_millis().to_string()) {
            Ok(d) => d.with_timezone(&Local),
            Err(_) => Local::now(),
        };
        let time = dt.format("%Y-%m-%d %I:%M:%S%.3f %p");
        writeln!(f, "{}|{}|{}|{}", time, level, target, record.args(),)
    });

    builder
}

pub fn configure(level: &str, logger: &mut Builder) -> Result<(), ShellError> {
    let level = match level {
        "error" => LevelFilter::Error,
        "warn" => LevelFilter::Warn,
        "info" => LevelFilter::Info,
        "debug" => LevelFilter::Debug,
        "trace" => LevelFilter::Trace,
        _ => LevelFilter::Warn,
    };

    logger.filter_module("nu", level);

    if let Ok(s) = std::env::var("RUST_LOG") {
        logger.parse_filters(&s);
    }

    Ok(())
}

// pub fn trace_filters(app: &App, logger: &mut Builder) -> Result<(), ShellError> {
//     if let Some(filters) = app.develop() {
//         filters.into_iter().filter_map(Result::ok).for_each(|name| {
//             logger.filter_module(&name, LevelFilter::Trace);
//         })
//     }

//     Ok(())
// }

// pub fn debug_filters(app: &App, logger: &mut Builder) -> Result<(), ShellError> {
//     if let Some(filters) = app.debug() {
//         filters.into_iter().filter_map(Result::ok).for_each(|name| {
//             logger.filter_module(&name, LevelFilter::Debug);
//         })
//     }

//     Ok(())
// }

fn colored_level<'a>(
    style: &'a mut pretty_env_logger::env_logger::fmt::Style,
    level: Level,
) -> pretty_env_logger::env_logger::fmt::StyledValue<'a, &'static str> {
    match level {
        Level::Trace => style.set_color(Color::Magenta).value("TRACE"),
        Level::Debug => style.set_color(Color::Blue).value("DEBUG"),
        Level::Info => style.set_color(Color::Green).value("INFO "),
        Level::Warn => style.set_color(Color::Yellow).value("WARN "),
        Level::Error => style.set_color(Color::Red).value("ERROR"),
    }
}

static MAX_MODULE_WIDTH: AtomicUsize = AtomicUsize::new(0);

fn max_target_width(target: &str) -> usize {
    let max_width = MAX_MODULE_WIDTH.load(Ordering::Relaxed);
    if max_width < target.len() {
        MAX_MODULE_WIDTH.store(target.len(), Ordering::Relaxed);
        target.len()
    } else {
        max_width
    }
}

struct Padded<T> {
    value: T,
    width: usize,
}

impl<T: fmt::Display> fmt::Display for Padded<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{: <width$}", self.value, width = self.width)
    }
}
