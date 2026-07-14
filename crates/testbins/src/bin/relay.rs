use indoc::printdoc;
use lexopt::prelude::*;
use std::io;

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
            Relays anything received on stdin to stdout

            Usage: relay

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

    io::copy(&mut io::stdin().lock(), &mut io::stdout().lock())
        .expect("failed to copy stdin to stdout");
}
