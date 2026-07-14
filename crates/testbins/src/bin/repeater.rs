use indoc::printdoc;
use lexopt::prelude::*;
use std::io::{self, Write};

pub struct Args {
    help: bool,
    letter: String,
    count: u64,
}

impl Args {
    pub fn parse() -> Result<Self, lexopt::Error> {
        let mut help = false;
        let mut values = Vec::new();

        let mut parser = lexopt::Parser::from_env();
        while let Some(arg) = parser.next()? {
            match arg {
                Short('h') | Long("help") => help = true,
                Value(value) => values.push(value),
                arg => return Err(arg.unexpected()),
            }
        }

        let mut values = values.into_iter();
        Ok(Self {
            help,
            letter: values
                .next()
                .map(|value| value.parse())
                .transpose()?
                .unwrap_or_default(),
            count: values
                .next()
                .map(|value| value.parse())
                .transpose()?
                .unwrap_or_default(),
        })
    }

    pub fn help() {
        printdoc! {r#"
            Repeat a string or char N times

            Usage: repeater <LETTER> <COUNT>

            Arguments:
              <LETTER>  String or character to repeat
              <COUNT>   Number of times to repeat it

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
    for _ in 0..args.count {
        let _ = write!(stdout, "{}", args.letter);
    }
    let _ = stdout.flush();
}
