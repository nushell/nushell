use clap::{App, Arg};
use log::LevelFilter;
use std::error::Error;
use std::fs::File;
use std::io::{prelude::*, BufReader};

fn main() -> Result<(), Box<dyn Error>> {
    let matches = App::new("nushell")
        .version(clap::crate_version!())
        .arg(
            Arg::with_name("loglevel")
                .short("l")
                .long("loglevel")
                .value_name("LEVEL")
                .possible_values(&["error", "warn", "info", "debug", "trace"])
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
        .get_matches();

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
            let pipelines: Vec<String> = values.map(|x| x.to_string()).collect();
            futures::executor::block_on(nu_cli::run_vec_of_pipelines(
                pipelines,
                matches.is_present("stdin"),
            ))?;
            return Ok(());
        }
    }

    match matches.value_of("script") {
        Some(script) => {
            let file = File::open(script)?;
            let reader = BufReader::new(file);
            let pipelines: Vec<String> = reader
                .lines()
                .filter_map(|x| {
                    if let Ok(x) = x {
                        if !x.starts_with('#') {
                            Some(x)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect();

            futures::executor::block_on(nu_cli::run_vec_of_pipelines(
                pipelines,
                matches.is_present("stdin"),
            ))?;
            return Ok(());
        }

        None => {
            println!(
                "Welcome to Nushell {} (type 'help' for more info)",
                clap::crate_version!()
            );
            futures::executor::block_on(nu_cli::cli())?;
        }
    }

    Ok(())
}
