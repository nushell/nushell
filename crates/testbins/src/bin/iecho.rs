use indoc::printdoc;
use lexopt::prelude::*;
use std::io::{self, Write};

pub struct Args {
    help: bool,
    args: Vec<String>,
}

impl Args {
    pub fn parse() -> Result<Self, lexopt::Error> {
        let mut args = Self {
            help: false,
            args: Vec::new(),
        };

        let mut parser = lexopt::Parser::from_env();
        while let Some(arg) = parser.next()? {
            match arg {
                Short('h') | Long("help") => args.help = true,
                Value(value) => args.args.push(value.parse()?),
                arg => return Err(arg.unexpected()),
            }
        }

        Ok(args)
    }

    pub fn help() {
        printdoc! {r#"
            Another type of echo that outputs a parameter per line, looping infinitely

            Usage: iecho [ARGS...]

            Arguments:
              [ARGS...]  Values to print repeatedly, one per line

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

    let mut stdout = io::stdout();
    let _ = args
        .args
        .iter()
        .cycle()
        .try_for_each(|value| writeln!(stdout, "{value}"));
}
