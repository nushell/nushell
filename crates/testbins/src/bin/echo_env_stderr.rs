use std::io::Stderr;

use indoc::printdoc;
use lexopt::prelude::*;

pub struct Args {
    help: bool,
    keys: Vec<String>,
}

impl Args {
    pub fn parse() -> Result<Self, lexopt::Error> {
        let mut args = Self {
            help: false,
            keys: Vec::new(),
        };

        let mut parser = lexopt::Parser::from_env();
        while let Some(arg) = parser.next()? {
            match arg {
                Short('h') | Long("help") => args.help = true,
                Value(value) => args.keys.push(value.parse()?),
                arg => return Err(arg.unexpected()),
            }
        }

        Ok(args)
    }

    pub fn help() {
        printdoc! {r#"
            Echo's value of env keys from args to stderr

            Usage: echo_env_stderr [KEYS...]

            Arguments:
              [KEYS...]  Environment variable names to echo to stderr

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

    testbins::echo_env::<Stderr>(args.keys);
}
