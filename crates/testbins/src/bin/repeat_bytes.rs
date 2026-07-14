use indoc::printdoc;
use lexopt::prelude::*;
use std::io::{self, Write};

pub struct Args {
    help: bool,
    repeats: Vec<RepeatBytesArg>,
}

pub struct RepeatBytesArg {
    bytes_hex: String,
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

        let mut repeats = Vec::new();
        let mut values = values.into_iter();
        while let (Some(bytes_hex), Some(count)) = (values.next(), values.next()) {
            repeats.push(RepeatBytesArg {
                bytes_hex: bytes_hex.parse()?,
                count: count.parse()?,
            });
        }

        Ok(Self { help, repeats })
    }

    pub fn help() {
        printdoc! {r#"
            A version of repeater that can output binary data, even null bytes

            Usage: repeat_bytes [BYTES_HEX COUNT]...

            Arguments:
              [BYTES_HEX COUNT]...  Hex bytes and repeat count pairs

            Options:
              -h, --help            Show this help text
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

    for repeat in args.repeats {
        let bytes: Vec<u8> = (0..repeat.bytes_hex.len())
            .step_by(2)
            .map(|i| {
                u8::from_str_radix(&repeat.bytes_hex[i..i + 2], 16)
                    .expect("binary string is valid hexadecimal")
            })
            .collect();

        for _ in 0..repeat.count {
            stdout
                .write_all(&bytes)
                .expect("writing to stdout must not fail");
        }
    }

    let _ = stdout.flush();
}
