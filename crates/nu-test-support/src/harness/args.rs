use std::num::NonZeroUsize;

use kitest::formatter::common::color::ColorSetting;

#[derive(Debug)]
pub struct Args {
    pub color: ColorSetting,
    pub exact: bool,
    pub filter: Vec<String>,
    pub format: Format,
    pub help: bool,
    pub ignored: bool,
    pub include_ignored: bool,
    pub list: bool,
    pub no_capture: bool,
    pub skip: Vec<String>,
    pub test_threads: Option<NonZeroUsize>,
}

#[derive(Debug)]
pub enum Format {
    Pretty,
    Terse,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            color: ColorSetting::Automatic,
            exact: false,
            filter: Vec::new(),
            format: Format::Pretty,
            help: false,
            ignored: false,
            include_ignored: false,
            list: false,
            no_capture: false,
            skip: Vec::new(),
            test_threads: None,
        }
    }
}

impl Args {
    pub fn parse() -> Result<Args, lexopt::Error> {
        use lexopt::prelude::*;

        let mut args = Args::default();
        let mut parser = lexopt::Parser::from_env();

        fn parse_flag(parser: &mut lexopt::Parser, flag: &mut bool) -> Result<(), lexopt::Error> {
            let _: () = match parser.optional_value() {
                None => *flag = true,
                Some(value) => *flag = value.parse()?,
            };
            Ok(())
        }

        while let Some(arg) = parser.next()? {
            match arg {
                Long("color") => {
                    let color = parser.value()?.string()?;
                    match color.as_str() {
                        "auto" | "automatic" => args.color = ColorSetting::Automatic,
                        "always" => args.color = ColorSetting::Always,
                        "never" => args.color = ColorSetting::Never,
                        _ => todo!(),
                    }
                }
                Long("exact") => parse_flag(&mut parser, &mut args.exact)?,
                Value(value) => args.filter.push(value.parse()?),
                Long("format") => {
                    let color: String = parser.value()?.parse()?;
                    match color.as_str() {
                        "pretty" => args.format = Format::Pretty,
                        "terse" => args.format = Format::Terse,
                        _ => todo!(),
                    }
                }
                Long("help") => parse_flag(&mut parser, &mut args.help)?,
                Long("ignored") => parse_flag(&mut parser, &mut args.ignored)?,
                Long("include-ignored") => parse_flag(&mut parser, &mut args.include_ignored)?,
                Long("list") => parse_flag(&mut parser, &mut args.list)?,
                Long("nocapture" | "no-capture") => parse_flag(&mut parser, &mut args.no_capture)?,
                Long("skip") => args.skip.push(parser.value()?.parse()?),
                Long("test-threads") => args.test_threads = Some(parser.value()?.parse()?),
                arg => return Err(arg.unexpected()),
            }
        }

        Ok(args)
    }

    #[rustfmt::skip]
    pub fn help() {
        indoc::printdoc! {r#"
            nu-test-support test harness (kitest based)

            Usage: [OPTIONS] [FILTERS...]

            Arguments:
              [OPTIONS]     Settings that adjust how the test binary runs
              [FILTERS...]  Names or patterns of tests to run

            Options:
              --color <auto|always|never>  Control colored output
              --exact                      Match filters exactly
              --format <pretty|terse>      Choose output style
              --help                       Show this help text
              --ignored                    Run only ignored tests
              --include-ignored            Include ignored tests
              --list                       List tests without running them
              --nocapture                  Print test output directly
              --skip <FILTER>              Skip matching tests, can be used multiple times
              --test-threads <N>           Number of test threads to use, default is {default_thread_count}

            Test Attributes
              #[test]                      Mark a function as a test. Must take no arguments.
              #[should_panic]              Test passes only if it panics.
                                           Can check the panic message with #[should_panic(expected = "foo")].
              #[ignore]                    Skip this test in normal runs. Use --ignored to run it.
              #[exp(option = true|false)]  Set an experimental option for this test.
                                           For the key import an `ExperimentalOption` and set it to
                                           true or false to enable or disable it.
              #[env(KEY = "value")]        Set environment variables for this test.
              #[serial]                    Run this test serially, with no other tests at the same time.
            "#, 
            default_thread_count = *super::DEFAULT_THREAD_COUNT
        };
    }
}
