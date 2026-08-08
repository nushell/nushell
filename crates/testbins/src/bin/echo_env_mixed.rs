use std::io::{Stderr, Stdout};
use std::str::FromStr;

use indoc::printdoc;
use lexopt::prelude::*;

pub struct Args {
    help: bool,
    mixed_type: MixedType,
    first_key: String,
    second_key: String,
}

pub enum MixedType {
    OutErr,
    ErrOut,
}

impl FromStr for MixedType {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "out-err" => Ok(Self::OutErr),
            "err-out" => Ok(Self::ErrOut),
            _ => Err("mixed type must be `out-err` or `err-out`"),
        }
    }
}

impl Args {
    pub fn parse() -> Result<Self, lexopt::Error> {
        let mut help = false;
        let mut values = Vec::new();

        let mut parser = lexopt::Parser::from_env();
        while let Some(arg) = parser.next()? {
            match arg {
                Short('h') | Long("help") => help = true,
                Value(value) => values.push(value.string()?),
                arg => return Err(arg.unexpected()),
            }
        }

        match values.as_slice() {
            _ if help => Ok(Self {
                help,
                mixed_type: MixedType::OutErr,
                first_key: String::new(),
                second_key: String::new(),
            }),
            [mixed_type, first_key, second_key] => Ok(Self {
                help,
                mixed_type: mixed_type.parse()?,
                first_key: first_key.clone(),
                second_key: second_key.clone(),
            }),
            [.., _, _, _, _] => Err("unexpected extra argument".into()),
            _ => {
                Err("missing arguments: expected <out-err|err-out> <FIRST_KEY> <SECOND_KEY>".into())
            }
        }
    }

    pub fn help() {
        printdoc! {r#"
            Mix echo of env keys from input

            Usage: echo_env_mixed <out-err|err-out> <FIRST_KEY> <SECOND_KEY>

            Arguments:
              <out-err|err-out>  Whether stdout or stderr is written first
              <FIRST_KEY>        First environment variable name
              <SECOND_KEY>       Second environment variable name

            Options:
              -h, --help         Show this help text
        "#};
    }
}

fn main() {
    let args = testbins::parse_args(Args::parse());
    if args.help {
        Args::help();
        testbins::exit_help();
    }

    match args.mixed_type {
        MixedType::OutErr => {
            testbins::echo_one_env::<Stdout>(&args.first_key);
            testbins::echo_one_env::<Stderr>(&args.second_key);
        }
        MixedType::ErrOut => {
            testbins::echo_one_env::<Stderr>(&args.first_key);
            testbins::echo_one_env::<Stdout>(&args.second_key);
        }
    }
}
