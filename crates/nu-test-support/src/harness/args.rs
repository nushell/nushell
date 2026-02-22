use std::{io::stdout, num::NonZeroUsize};

use kitest::formatter::common::color::ColorSetting;

#[derive(Debug)]
pub struct Args {
    pub color: ColorSetting,
    pub exact: bool,
    pub filter: Vec<String>,
    pub format: Format,
    pub help: bool,
    pub ignored: bool,
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
        use std::io::Write;

        let mut out = stdout();

        macro_rules! line {
            () => {{ let _ = ::std::writeln!(out); }};
            ($fmt:expr) => {{ let _ = ::std::writeln!(out, $fmt); }};
            ($fmt:expr, $($args:tt)*) => {{ let _ = ::std::writeln!(out, $fmt, $($args)*); }};
        }

        line!("nu-test-support test harness (kitest based)");
        line!();
        line!("Usage: [OPTIONS] [FILTERS...]");
        line!();
        line!("Arguments:");
        line!("  [OPTIONS]     Settings that adjust how the test binary runs");
        line!("  [FILTERS...]  Names or patterns of tests to run");
        line!();
        line!("Options:");
        line!("  --color <auto|always|never>  Control colored output");
        line!("  --exact                      Match filters exactly");
        line!("  --format <pretty|terse>      Choose output style");
        line!("  --help                       Show this help text");
        line!("  --ignored                    Run only ignored tests");
        line!("  --list                       List tests without running them");
        line!("  --nocapture                  Print test output directly");
        line!("  --skip <FILTER>              Skip matching tests, can be used multiple times");
        line!("  --test-threads <N>           Number of test threads to use, default is {}", *super::DEFAULT_THREAD_COUNT);
        line!();
        line!("Test Attributes:");
        line!("  #[test]                      Mark a function as a test. Must take no arguments.");
        line!("  #[should_panic]              Test passes only if it panics."); 
        line!("                               Can check the panic message with #[should_panic(expected = \"foo\")].");
        line!("  #[ignore]                    Skip this test in normal runs. Use --ignored to run it.");
        line!("  #[exp(option = true|false)]  Set an experimental option for this test.");
        line!("                               For the key import an `ExperimentalOption` and set it to");
        line!("                               true or false to enable or disable it.");
        line!("  #[env(KEY = \"value\")]        Set environment variables for this test.");
        line!("  #[serial]                    Run this test serially, with no other tests at the same time.");
    }
}
