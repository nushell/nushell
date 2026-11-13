use indoc::printdoc;
use lexopt::prelude::*;

pub struct Args {
    help: bool,
    exit_code: Option<i32>,
}

impl Args {
    pub fn parse() -> Result<Self, lexopt::Error> {
        let mut args = Self {
            help: false,
            exit_code: None,
        };

        let mut parser = lexopt::Parser::from_env();
        while let Some(arg) = parser.next()? {
            match arg {
                Short('h') | Long("help") => args.help = true,
                Value(value) => {
                    if args.exit_code.is_none() {
                        args.exit_code = Some(value.parse()?);
                    }
                }
                arg => return Err(arg.unexpected()),
            }
        }

        Ok(args)
    }

    pub fn help() {
        printdoc! {r#"
            Exits with failure code <c>, if not given, fail with code 1

            Usage: fail [EXIT_CODE]

            Arguments:
              [EXIT_CODE]  Exit code to return, defaults to 1

            Options:
              -h, --help   Show this help text
        "#};
    }
}

fn main() {
    let args = testbins::parse_args(Args::parse());
    if args.help {
        Args::help();
        testbins::exit_help();
    }

    testbins::fail(args.exit_code.unwrap_or(1));
}
