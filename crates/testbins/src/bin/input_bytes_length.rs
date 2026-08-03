use indoc::printdoc;
use lexopt::prelude::*;
use std::io::{self, Read};

pub struct Args {
    help: bool,
}

impl Args {
    pub fn parse() -> Result<Self, lexopt::Error> {
        let mut args = Self { help: false };

        let mut parser = lexopt::Parser::from_env();
        while let Some(arg) = parser.next()? {
            match arg {
                Short('h') | Long("help") => args.help = true,
                arg => return Err(arg.unexpected()),
            }
        }

        Ok(args)
    }

    pub fn help() {
        printdoc! {r#"
            Prints the number of bytes received on stdin

            Usage: input_bytes_length

            Options:
              -h, --help  Show this help text
        "#};
    }
}

fn main() {
    let args = testbins::parse_args(Args::parse());
    if args.help {
        Args::help();
        testbins::exit_help();
    }

    let stdin = io::stdin();
    let count = stdin.lock().bytes().count();

    println!("{count}");
}
