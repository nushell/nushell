use clap::{App, Arg, ArgMatches};
use itertools::Itertools;
use log::LevelFilter;
use nu_cli::{config, create_default_context};
use nu_command::utils::test_bins as binaries;
use nu_engine::{script, EvaluationContext};
use nu_protocol::{ConfigPath, NuScript, RunScriptOptions, UntaggedValue, Value};
use nu_source::{Tag, Text};
use std::{error::Error, path::PathBuf, sync::atomic::Ordering};

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

    init_logger(&matches)?;

    let ctx = create_context_from_args(&matches)?;

    // Execute nu according to matches
    if let Some(values) = matches.values_of("commands") {
        // Execute commands
        let commands: String = values.intersperse("\n").collect();
        let cmds_as_script = NuScript::Content(commands);
        let options = script_options_from_matches(&matches);
        futures::executor::block_on(script::run_script(cmds_as_script, &options, &ctx));
    } else if let Some(filepath) = matches.value_of("script") {
        // Execute script
        let script = NuScript::File(PathBuf::from(filepath));
        let options = script_options_from_matches(&matches);
        futures::executor::block_on(script::run_script(script, &options, &ctx));
    } else {
        // No matches
        // Go into cli mode
        #[cfg(feature = "rustyline-support")]
        {
            futures::executor::block_on(nu_cli::cli(ctx))?;
        }
        #[cfg(not(feature = "rustyline-support"))]
        {
            println!("Nushell needs the 'rustyline-support' feature for CLI support");
        }
    }

    Ok(())
}

fn script_options_from_matches(matches: &ArgMatches) -> RunScriptOptions {
    RunScriptOptions::default().with_stdin(matches.is_present("stdin"))
}

fn create_context_from_args(matches: &ArgMatches) -> Result<EvaluationContext, Box<dyn Error>> {
    //TODO interactive should be derived from current shell
    //TODO stop passing true
    let ctx = create_default_context(true)?;

    if !matches.is_present("skip-plugins") {
        register_plugins(&ctx);
    }

    configure_ctrl_c(&ctx)?;

    if let Some(cfg) = matches.value_of("config-file") {
        futures::executor::block_on(load_cfg_as_global_cfg(&ctx, PathBuf::from(cfg)));
    } else {
        futures::executor::block_on(load_global_cfg(&ctx));
    }

    Ok(ctx)
}

async fn load_cfg_as_global_cfg(context: &EvaluationContext, path: PathBuf) {
    if let Err(err) = context.load_config(&ConfigPath::Global(path.clone())).await {
        context.host.lock().print_err(err, &Text::empty());
    } else {
        //TODO current commands assume to find path to global cfg file under config-path
        //TODO use newly introduced nuconfig::file_path instead
        context.scope.add_var(
            "config-path",
            UntaggedValue::filepath(path).into_untagged_value(),
        );
    }
}

async fn load_global_cfg(context: &EvaluationContext) {
    match config::default_path() {
        Ok(path) => {
            load_cfg_as_global_cfg(context, path).await;
        }
        Err(e) => {
            context.host.lock().print_err(e, &Text::from(""));
        }
    }
}

fn configure_ctrl_c(_context: &EvaluationContext) -> Result<(), Box<dyn Error>> {
    #[cfg(feature = "ctrlc")]
    {
        let cc = _context.ctrl_c.clone();

        ctrlc::set_handler(move || {
            cc.store(true, Ordering::SeqCst);
        })?;

        if _context.ctrl_c.load(Ordering::SeqCst) {
            _context.ctrl_c.store(false, Ordering::SeqCst);
        }
    }

    Ok(())
}

fn register_plugins(context: &EvaluationContext) {
    //TODO we should probably report the error here
    if let Ok(plugins) = nu_engine::plugin::build_plugin::scan(search_paths()) {
        context.add_commands(
            plugins
                .into_iter()
                .filter(|p| !context.is_command_registered(p.name()))
                .collect(),
        );
    }
}

fn search_paths() -> Vec<std::path::PathBuf> {
    use std::env;

    let mut search_paths = Vec::new();

    // Automatically add path `nu` is in as a search path
    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            search_paths.push(exe_dir.to_path_buf());
        }
    }

    if let Ok(config) = nu_data::config::config(Tag::unknown()) {
        if let Some(Value {
            value: UntaggedValue::Table(pipelines),
            ..
        }) = config.get("plugin_dirs")
        {
            for pipeline in pipelines {
                if let Ok(plugin_dir) = pipeline.as_string() {
                    search_paths.push(PathBuf::from(plugin_dir));
                }
            }
        }
    }
    search_paths
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

fn init_logger(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
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

    builder.try_init()?;
    Ok(())
}
