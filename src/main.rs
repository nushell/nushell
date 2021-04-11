use clap::{App, Arg, ArgMatches};
use nu_cli::Options;
use nu_command::utils::test_bins as binaries;
mod init;

use init::init_from_args;
use nu_engine::script;
use nu_protocol::{NuScript, RunScriptOptions};
use std::{error::Error, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
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

    if executed_test_bin(&matches) {
        return Ok(());
    }

    let ctx = init_from_args(&matches)?;

    // Execute nu according to matches
    if let Some(values) = matches.values_of("commands") {
        // Execute commands
        let options = script_options_from_matches(&matches);
        for cmd in values {
            script::run_script(NuScript::Content(cmd.to_string()), &options, &ctx);
        }
    } else if let Some(filepath) = matches.value_of("script") {
        // Execute script
        let script = NuScript::File(PathBuf::from(filepath));
        let options = script_options_from_matches(&matches);
        script::run_script(script, &options, &ctx);
    } else {
        // No matches
        // Go into cli mode
        #[cfg(feature = "rustyline-support")]
        {
            let ops = cli_options_from_matches(&matches);
            nu_cli::cli(ctx, ops)?;
        }
        #[cfg(not(feature = "rustyline-support"))]
        {
            println!("Nushell needs the 'rustyline-support' feature for CLI support");
        }
    }

    Ok(())
}

fn cli_options_from_matches(matches: &ArgMatches) -> Options {
    let mut ops = Options::default();
    ops.save_history(!matches.is_present("no-history"));

    ops
}

fn script_options_from_matches(matches: &ArgMatches) -> RunScriptOptions {
    RunScriptOptions::default()
        .with_stdin(matches.is_present("stdin"))
        //We always exit on error
        .exit_on_error(true)
}

fn executed_test_bin(matches: &ArgMatches) -> bool {
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
        true
    } else {
        false
    }
}
