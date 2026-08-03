use indoc::printdoc;
use lexopt::prelude::*;
use std::io::{self, Write};

pub struct Args {
    help: bool,
    paths: Vec<String>,
}

impl Args {
    pub fn parse() -> Result<Self, lexopt::Error> {
        let mut args = Self {
            help: false,
            paths: Vec::new(),
        };

        let mut parser = lexopt::Parser::from_env();
        while let Some(arg) = parser.next()? {
            match arg {
                Short('h') | Long("help") => args.help = true,
                Value(value) => args.paths.push(value.parse()?),
                arg => return Err(arg.unexpected()),
            }
        }

        Ok(args)
    }

    pub fn help() {
        printdoc! {r#"
            Cross platform cat (open a file, print the contents) using read() and write_all() / binary

            Usage: meowb <PATHS...>

            Arguments:
              <PATHS...>  Files to read as bytes

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

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    for path in args.paths {
        let buf = std::fs::read(path).expect("Expected a filepath");
        handle.write_all(&buf).expect("failed to write to stdout");
    }
}
