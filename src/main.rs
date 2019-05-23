#![feature(crate_visibility_modifier)]
#![feature(in_band_lifetimes)]

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
    crate::cli::cli()
}
