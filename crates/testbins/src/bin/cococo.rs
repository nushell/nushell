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
            Cross platform echo using println!()

            Usage: cococo [ARGS...]

            Arguments:
              [ARGS...]  Values to echo separated by spaces

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

    if args.args.is_empty() {
        // Write back out all the arguments passed
        // if given at least 1 instead of chickens
        // speaking co co co.
        println!("cococo");
    } else {
        println!("{}", args.args.join(" "));
    }
}
