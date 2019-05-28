#![feature(crate_visibility_modifier)]
#![feature(in_band_lifetimes)]
#![feature(async_await)]
#![feature(try_trait)]
#![feature(bind_by_move_pattern_guards)]

mod cli;
mod commands;
mod context;
mod env;
mod errors;
mod evaluate;
mod format;
mod object;
mod parser;
mod prelude;
mod shell;
mod stream;

use std::error::Error;

fn main() -> Result<(), Box<Error>> {
    pretty_env_logger::init();
    futures::executor::block_on(crate::cli::cli())?;
    Ok(())
}
