use super::App;
use log::LevelFilter;
use nu_errors::ShellError;
use pretty_env_logger::env_logger::Builder;

pub fn logger(f: impl FnOnce(&mut Builder) -> Result<(), ShellError>) -> Result<(), ShellError> {
    let mut builder = pretty_env_logger::formatted_builder();
    f(&mut builder)?;
    let _ = builder.try_init();
    Ok(())
}

pub fn configure(app: &App, logger: &mut Builder) -> Result<(), ShellError> {
    if let Some(level) = app.loglevel() {
        let level = match level.as_deref() {
            Ok("error") => LevelFilter::Error,
            Ok("warn") => LevelFilter::Warn,
            Ok("info") => LevelFilter::Info,
            Ok("debug") => LevelFilter::Debug,
            Ok("trace") => LevelFilter::Trace,
            Ok(_) | Err(_) => LevelFilter::Warn,
        };

        logger.filter_module("nu", level);
    };

    if let Ok(s) = std::env::var("RUST_LOG") {
        logger.parse_filters(&s);
    }

    Ok(())
}

pub fn trace_filters(app: &App, logger: &mut Builder) -> Result<(), ShellError> {
    if let Some(filters) = app.develop() {
        filters.into_iter().filter_map(Result::ok).for_each(|name| {
            logger.filter_module(&name, LevelFilter::Trace);
        })
    }

    Ok(())
}

pub fn debug_filters(app: &App, logger: &mut Builder) -> Result<(), ShellError> {
    if let Some(filters) = app.debug() {
        filters.into_iter().filter_map(Result::ok).for_each(|name| {
            logger.filter_module(&name, LevelFilter::Debug);
        })
    }

    Ok(())
}
