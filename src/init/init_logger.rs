use clap::ArgMatches;
use log::LevelFilter;
use nu_errors::ShellError;

pub fn init_logger(matches: &ArgMatches) -> Result<(), ShellError> {
    let loglevel = match matches.value_of("loglevel") {
        None => LevelFilter::Warn,
        Some("error") => LevelFilter::Error,
        Some("warn") => LevelFilter::Warn,
        Some("info") => LevelFilter::Info,
        Some("debug") => LevelFilter::Debug,
        Some("trace") => LevelFilter::Trace,
        _ => unreachable!(),
    };

    let mut builder = pretty_env_logger::formatted_builder();

    if let Ok(s) = std::env::var("RUST_LOG") {
        builder.parse_filters(&s);
    }

    builder.filter_module("nu", loglevel);

    //TODO the following 2 match statements seem to duplicate above logic?
    match matches.values_of("develop") {
        None => {}
        Some(values) => {
            for item in values {
                builder.filter_module(&format!("nu::{}", item), LevelFilter::Trace);
            }
        }
    }

    match matches.values_of("debug") {
        None => {}
        Some(values) => {
            for item in values {
                builder.filter_module(&format!("nu::{}", item), LevelFilter::Debug);
            }
        }
    }

    builder.try_init().map_err(|e| {
        ShellError::untagged_runtime_error(format!(
            "There was a problem initializing the logger. The error was: {:?}",
            e
        ))
    })?;
    Ok(())
}
