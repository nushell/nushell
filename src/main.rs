mod config_files;
mod eval_file;
mod logger;
mod prompt_update;
mod reedline_config;
mod repl;
mod utils;

// mod fuzzy_completion;

#[cfg(test)]
mod tests;

use miette::Result;
use nu_command::create_default_context;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

fn main() -> Result<()> {
    // miette::set_panic_hook();
    let miette_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |x| {
        crossterm::terminal::disable_raw_mode().expect("unable to disable raw mode");
        miette_hook(x);
    }));

    // Get initial current working directory.
    let init_cwd = utils::get_init_cwd();
    let mut engine_state = create_default_context(&init_cwd);

    // TODO: make this conditional in the future
    // Ctrl-c protection section
    let ctrlc = Arc::new(AtomicBool::new(false));
    let handler_ctrlc = ctrlc.clone();
    let engine_state_ctrlc = ctrlc.clone();

    ctrlc::set_handler(move || {
        handler_ctrlc.store(true, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    engine_state.ctrlc = Some(engine_state_ctrlc);
    // End ctrl-c protection section

    if let Some(path) = std::env::args().nth(1) {
        eval_file::evaluate(path, init_cwd, &mut engine_state)
    } else {
        repl::evaluate(ctrlc, &mut engine_state)
    }
}
