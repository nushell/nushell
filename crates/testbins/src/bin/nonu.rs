use indoc::printdoc;
use lexopt::prelude::*;

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
            Cross platform echo but concats arguments without space and NO newline

            Usage: nonu [ARGS...]

            Arguments:
              [ARGS...]  Values to print without separators or trailing newline

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

    for arg in args.args {
        print!("{arg}");
    }
}
