use log::{Level, LevelFilter, SetLoggerError};
use nu_protocol::ShellError;
use simplelog::{
    Color, ColorChoice, Config, ConfigBuilder, LevelPadding, TermLogger, TerminalMode, WriteLogger,
    format_description,
};

use std::{fs::File, path::Path, str::FromStr};

pub enum LogTarget {
    Stdout,
    Stderr,
    Mixed,
    File,
}

impl From<&str> for LogTarget {
    fn from(s: &str) -> Self {
        match s {
            "stdout" => Self::Stdout,
            "mixed" => Self::Mixed,
            "file" => Self::File,
            _ => Self::Stderr,
        }
    }
}

pub fn logger(
    f: impl FnOnce(&mut ConfigBuilder) -> (LevelFilter, LogTarget),
) -> Result<(), ShellError> {
    let mut builder = ConfigBuilder::new();
    let (level, target) = f(&mut builder);

    let config = builder.build();
    let _ = match target {
        LogTarget::Stdout => {
            TermLogger::init(level, config, TerminalMode::Stdout, ColorChoice::Auto)
        }
        LogTarget::Mixed => TermLogger::init(level, config, TerminalMode::Mixed, ColorChoice::Auto),
        LogTarget::File => {
            let pid = std::process::id();
            let mut path = std::env::temp_dir();
            path.push(format!("nu-{pid}.log"));

            set_write_logger(level, config, &path)
        }
        _ => TermLogger::init(level, config, TerminalMode::Stderr, ColorChoice::Auto),
    };

    Ok(())
}

fn set_write_logger(level: LevelFilter, config: Config, path: &Path) -> Result<(), SetLoggerError> {
    // Use TermLogger instead if WriteLogger is not available
    if let Ok(file) = File::create(path) {
        WriteLogger::init(level, config, file)
    } else {
        let default_logger =
            TermLogger::init(level, config, TerminalMode::Stderr, ColorChoice::Auto);

        if default_logger.is_ok() {
            log::warn!("failed to init WriteLogger, use TermLogger instead");
        }

        default_logger
    }
}

pub struct Filters {
    pub include: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
}

pub fn configure(
    level: &str,
    target: &str,
    filters: Filters,
    builder: &mut ConfigBuilder,
) -> (LevelFilter, LogTarget) {
    let level = match Level::from_str(level) {
        Ok(level) => level,
        Err(_) => Level::Warn,
    };

    // Add allowed module filter
    if let Some(include) = filters.include {
        for filter in include {
            builder.add_filter_allow(filter);
        }
    } else {
        builder.add_filter_allow_str("nu");
    }

    // Add ignored module filter
    if let Some(exclude) = filters.exclude {
        for filter in exclude {
            builder.add_filter_ignore(filter);
        }
    }

    // Set level padding
    builder.set_level_padding(LevelPadding::Right);

    // Custom time format
    builder.set_time_format_custom(format_description!(
        "[year]-[month]-[day] [hour repr:12]:[minute]:[second].[subsecond digits:3] [period]"
    ));

    // Show module path
    builder.set_target_level(LevelFilter::Error);

    // Don't show thread id
    builder.set_thread_level(LevelFilter::Off);

    let log_target = LogTarget::from(target);

    // Only TermLogger supports color output
    if let LogTarget::Stdout | LogTarget::Stderr | LogTarget::Mixed = log_target {
        Level::iter().for_each(|level| set_colored_level(builder, level));
    }

    (level.to_level_filter(), log_target)
}

fn set_colored_level(builder: &mut ConfigBuilder, level: Level) {
    let color = match level {
        Level::Trace => Color::Magenta,
        Level::Debug => Color::Blue,
        Level::Info => Color::Green,
        Level::Warn => Color::Yellow,
        Level::Error => Color::Red,
    };

    builder.set_level_color(level, Some(color));
}
