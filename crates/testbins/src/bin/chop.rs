use indoc::printdoc;
use lexopt::prelude::*;
use std::io::{self, BufRead, Write};

pub struct Args {
    help: bool,
    args: Vec<String>,
}

fn chopped(value: &str) -> &str {
    if value.is_empty() {
        value
    } else {
        &value[..value.len() - 1]
    }
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
            With no parameters, will chop a character off the end of each line

            Usage: chop [ARGS...]

            Arguments:
              [ARGS...]  Values to chop directly instead of reading stdin

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

    if !args.args.is_empty() {
        for arg in args.args {
            println!("{}", chopped(&arg));
        }
        return;
    }

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for given in stdin.lock().lines().map_while(Result::ok) {
        if writeln!(stdout, "{}", chopped(&given)).is_err() {
            break;
        }
    }
}
