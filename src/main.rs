#![feature(crate_visibility_modifier)]
#![feature(in_band_lifetimes)]
#![feature(async_await)]

mod cli;
mod commands;
mod context;
mod env;
mod errors;
mod format;
mod object;
mod parser;
mod prelude;
mod shell;
mod stream;

use std::error::Error;

fn main() -> Result<(), Box<Error>> {
    futures::executor::block_on(crate::cli::cli());
    Ok(())
}
