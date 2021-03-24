mod init_logger;
mod load_configs;
mod load_plugins;

use clap::ArgMatches;

use nu_engine::{basic_evaluation_context, EvaluationContext, FilesystemShellMode};
use nu_errors::ShellError;
use nu_source::Text;

use std::{path::PathBuf, sync::atomic::Ordering};

use self::init_logger::init_logger;
use self::load_configs::{load_cfg_as_global_cfg, load_global_cfg};
use self::load_plugins::load_plugins;

pub fn init_from_args(matches: &ArgMatches) -> Result<EvaluationContext, ShellError> {
    // Logger must be initialized before ctx creation so ctx creation is logged
    let logger_result = init_logger(matches);

    // Not being able to create a context is fatal. Return with err.
    let ctx = create_default_context(matches)?;

    // Ctrl_c err is not fatal. Just print err
    if let Err(e) = configure_ctrl_c(&ctx) {
        ctx.host.lock().print_err(e, &Text::empty());
    }

    // Logger err is not fatal. Print logger err afterwards
    if let Err(e) = logger_result {
        ctx.host.lock().print_err(e, &Text::empty());
    }

    if !matches.is_present("skip-plugins") {
        load_plugins(&ctx);
    }

    if let Some(cfg) = matches.value_of("config-file") {
        futures::executor::block_on(load_cfg_as_global_cfg(&ctx, PathBuf::from(cfg)));
    } else {
        futures::executor::block_on(load_global_cfg(&ctx));
    }

    Ok(ctx)
}

fn create_default_context(matches: &ArgMatches) -> Result<EvaluationContext, ShellError> {
    let mode = if matches.is_present("commands") || matches.is_present("script") {
        FilesystemShellMode::Script
    } else {
        FilesystemShellMode::Cli
    };

    let context = basic_evaluation_context(mode)?;

    context.add_commands(nu_command::all_cmds());

    Ok(context)
}

fn configure_ctrl_c(_context: &EvaluationContext) -> Result<(), ShellError> {
    #[cfg(feature = "ctrlc")]
    {
        let cc = _context.ctrl_c.clone();

        ctrlc::set_handler(move || {
            cc.store(true, Ordering::SeqCst);
        })
        .map_err(|e| {
            ShellError::untagged_runtime_error(format!(
                "Erroring configuring ctrl_c. Error was:\n{:?} ",
                e
            ))
        })?;

        if _context.ctrl_c.load(Ordering::SeqCst) {
            _context.ctrl_c.store(false, Ordering::SeqCst);
        }
    }

    Ok(())
}
