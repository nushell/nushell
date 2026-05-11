use log::{Level, LevelFilter, SetLoggerError};
use nu_protocol::ShellError;
use nu_protocol::shell_error::generic::GenericError;
use simplelog::{
    Color, ColorChoice, Config, ConfigBuilder, LevelPadding, TermLogger, TerminalMode, WriteLogger,
    format_description,
};

use std::{fs::File, path::Path, str::FromStr};

#[derive(Debug)]
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
    f: impl FnOnce(&mut ConfigBuilder) -> Result<(LevelFilter, LogTarget, Option<String>), ShellError>,
) -> Result<(), ShellError> {
    let mut builder = ConfigBuilder::new();
    let (level, target, custom_file) = f(&mut builder)?;

    let config = builder.build();
    let _ = match target {
        LogTarget::Stdout => {
            TermLogger::init(level, config, TerminalMode::Stdout, ColorChoice::Auto)
        }
        LogTarget::Mixed => TermLogger::init(level, config, TerminalMode::Mixed, ColorChoice::Auto),
        LogTarget::File => {
            // The configuration routine should already have enforced that a file path exists whenever the target is `File`.
            // But we should double‑check and turn a missing path into an error rather than panic.
            let file_path = if let Some(p) = custom_file.as_ref() {
                p
            } else {
                return Err(ShellError::Generic(GenericError::new_internal(
                    "logger misconfigured",
                    "log target is file but no path was provided",
                )));
            };

            let path = Path::new(file_path).to_path_buf();

            // ensure the file exists immediately
            let _ = std::fs::File::create(&path);

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
    custom_file: Option<&str>,
    filters: Filters,
    builder: &mut ConfigBuilder,
) -> Result<(LevelFilter, LogTarget, Option<String>), ShellError> {
    let level = match Level::from_str(level) {
        Ok(level) => level,
        Err(_) => Level::Info,
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

    // Require an explicit log file when the target is "file".
    if let LogTarget::File = log_target {
        if custom_file.is_none() {
            return Err(ShellError::Generic(GenericError::new_internal(
                "missing log file",
                "--log-target file requires --log-file",
            )));
        }
    } else if custom_file.is_some() {
        // If the target isn't file, providing a custom log file makes no sense.
        return Err(ShellError::Generic(GenericError::new_internal(
            "log file without file target",
            "--log-file requires --log-target file",
        )));
    }

    // Only TermLogger supports color output
    if let LogTarget::Stdout | LogTarget::Stderr | LogTarget::Mixed = log_target {
        Level::iter().for_each(|level| set_colored_level(builder, level));
    }

    Ok((
        level.to_level_filter(),
        log_target,
        custom_file.map(|s| s.to_string()),
    ))
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

#[cfg(test)]
mod tests {
    use super::*;
    use simplelog::ConfigBuilder;

    #[test]
    fn configure_requires_log_file_when_target_file() {
        let mut builder = ConfigBuilder::new();
        let filters = Filters {
            include: None,
            exclude: None,
        };
        let err = configure("info", "file", None, filters, &mut builder).unwrap_err();
        assert!(
            err.to_string().contains("requires --log-file")
                || err.to_string().contains("missing log file")
        );
    }

    #[test]
    fn configure_rejects_log_file_without_file_target() {
        let mut builder = ConfigBuilder::new();
        let filters = Filters {
            include: None,
            exclude: None,
        };
        let err = configure("info", "stderr", Some("/tmp/foo"), filters, &mut builder).unwrap_err();
        assert!(
            err.to_string().contains("requires --log-target file")
                || err.to_string().contains("log file without file target")
        );
    }

    #[test]
    fn configure_accepts_file_target_when_log_file_provided() {
        let mut builder = ConfigBuilder::new();
        let filters = Filters {
            include: None,
            exclude: None,
        };
        let res = configure("info", "file", Some("/tmp/foo"), filters, &mut builder);
        assert!(res.is_ok());
    }
}
