use nu_protocol::ShellError;
use std::{fs::File, str::FromStr};
use tracing::Level;
use tracing_subscriber::filter::LevelFilter;
pub enum LogTarget {
    Stdout,
    Stderr,
    File,
}
use std::io;

impl From<&str> for LogTarget {
    fn from(s: &str) -> Self {
        match s {
            "stdout" => Self::Stdout,
            "file" => Self::File,
            _ => Self::Stderr,
        }
    }
}

pub fn setup_logger(level: &str, target: &str) -> Result<(), ShellError> {
    let level = match Level::from_str(level) {
        Ok(level) => level,
        Err(_) => Level::WARN,
    };
    let builder = tracing_subscriber::fmt()
        .with_max_level(LevelFilter::from_level(level))
        .with_file(true)
        .with_line_number(true);

    // setup target.
    let log_target = LogTarget::from(target);
    match log_target {
        LogTarget::Stderr => builder.with_writer(io::stderr).init(),
        LogTarget::Stdout => builder.with_writer(io::stdout).init(),
        LogTarget::File => {
            let pid = std::process::id();
            let mut path = std::env::temp_dir();
            path.push(format!("nu-{pid}.log"));
            if let Ok(file) = File::create(path) {
                builder.with_writer(file).with_ansi(false).init()
            } else {
                builder.init()
            }
        }
    };
    Ok(())
}
