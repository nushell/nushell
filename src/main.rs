use clap::{App, Arg};
use log::LevelFilter;
use nu_cli::{create_default_context, NuScript, Options};
use nu_command::utils::test_bins as binaries;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let mut options = Options::new();

    let matches = App::new("nushell")
        .version(clap::crate_version!())
        .arg(
            Arg::with_name("config-file")
                .long("config-file")
                .help("custom configuration source file")
                .hidden(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("no-history")
                .hidden(true)
                .long("no-history")
                .multiple(false)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("loglevel")
                .short("l")
                .long("loglevel")
                .value_name("LEVEL")
                .possible_values(&["error", "warn", "info", "debug", "trace"])
                .takes_value(true),
        )
        .arg(
            Arg::with_name("skip-plugins")
                .hidden(true)
                .long("skip-plugins")
                .multiple(false)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("testbin")
                .hidden(true)
                .long("testbin")
                .value_name("TESTBIN")
                .possible_values(&[
                    "echo_env", "cococo", "iecho", "fail", "nonu", "chop", "repeater",
                ])
                .takes_value(true),
        )
        .arg(
            Arg::with_name("commands")
                .short("c")
                .long("commands")
                .multiple(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("develop")
                .long("develop")
                .multiple(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("debug")
                .long("debug")
                .multiple(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("stdin")
                .long("stdin")
                .multiple(false)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("script")
                .help("the nu script to run")
                .index(1),
        )
        .arg(
            Arg::with_name("args")
                .help("positional args (used by --testbin)")
                .index(2)
                .multiple(true),
        )
        .get_matches();

    if let Some(bin) = matches.value_of("testbin") {
        match bin {
            "echo_env" => binaries::echo_env(),
            "cococo" => binaries::cococo(),
            "iecho" => binaries::iecho(),
            "fail" => binaries::fail(),
            "nonu" => binaries::nonu(),
            "chop" => binaries::chop(),
            "repeater" => binaries::repeater(),
            _ => unreachable!(),
        }

        return Ok(());
    }

    options.config = matches
        .value_of("config-file")
        .map(std::ffi::OsString::from);
    options.stdin = matches.is_present("stdin");
    options.save_history = !matches.is_present("no-history");

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

    builder.try_init()?;

    match matches.values_of("commands") {
        None => {}
        Some(values) => {
            options.scripts = vec![NuScript::code(values)?];

            futures::executor::block_on(nu_cli::run_script_file(options))?;
            return Ok(());
        }
    }

    match matches.value_of("script") {
        Some(filepath) => {
            let filepath = std::ffi::OsString::from(filepath);

            options.scripts = vec![NuScript::source_file(filepath.as_os_str())?];

            futures::executor::block_on(nu_cli::run_script_file(options))?;
            return Ok(());
        }

        None => {
            let context = create_default_context(true)?;

            if !matches.is_present("skip-plugins") {
                let _ = nu_cli::register_plugins(&context);
            }

            #[cfg(feature = "rustyline-support")]
            {
                futures::executor::block_on(nu_cli::cli(context, options))?;
            }

            #[cfg(not(feature = "rustyline-support"))]
            {
                println!("Nushell needs the 'rustyline-support' feature for CLI support");
            }
        }
    }

    Ok(())
}
