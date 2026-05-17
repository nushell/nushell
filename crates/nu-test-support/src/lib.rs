#![expect(clippy::test_attr_in_doctest)]

//! Test support for the Nushell crates.
//!
//! This crate provides tools for testing Nushell crates, including support for both unit and
//! integration testing.
//! It offers a [custom test harness](#custom-test-harness) to control the environment tests run in, along with
//! [filesystem sandboxing](playground), utilities for
//! [executing and asserting nushell scripts](tester), and additional general helper functionality.
//!
//! # Custom Test Harness
//!
//! Running tests in specific environments is difficult with the default built-in test harness,
//! especially when it comes to serial execution, setting environment variables, or configuring
//! global state.
//! This crate provides a [custom test harness](harness) based on [kitest] to address these issues.
//! It works for both unit and integration tests, and most crates in nushell are already set up to
//! use it.
//! The harness behaves similarly to the regular test harness, so getting started does not require
//! special knowledge.
//!
//! ## Setup for Unit Tests
//!
//! In Cargo.toml of the crate:
//! ```custom,{class=language-toml}
//! [lib]
//! harness = false # important part
//! ```
//! This disables the built-in test harness in your library and requires a `main` function to
//! execute tests.
//! You can simply import the provided entry point:
//! ```
//! #[cfg(test)]
//! use nu_test_support::harness::main;
//! ```
//!
//! ## Setup for Integration Tests
//!
//! In Cargo.toml of the crate:
//! ```custom,{class=language-toml}
//! [package]
//! autotests = false # disable automatically found tests
//!
//! [[test]]
//! name = "tests"         # whatever name fits here
//! path = "tests/main.rs" # path to the main file
//! harness = false        # disable the test harness
//! ```
//! This disables autotests, so all integration tests must be defined manually.
//! All tests should live in the defined test binary as modules, and since the default harness is
//! disabled, the provided harness must be used.
//!
//! ## Using `#[test]` Macro
//!
//! To use the provided test harness and have it discover tests, a new test macro setup is required.
//! In the `main.rs` of the test:
//! ```
//! #[cfg(test)] // for unit tests, not required for integration tests
//! #[macro_use]
//! extern crate nu_test_support;
//! ```
//! This overrides the prelude macros with those from this crate, in particular the
//! [`test`](harness::macros::test) macro.
//! This allows test writers to keep using `#[test]` on test functions as usual.
//!
//! ## Configuring Test Environment
//!
//! When using the test harness, additional attributes are available that can be used together with
//! `#[test]` to control how tests are executed.
//!
//! - `#[serial]`
//!   Runs tests sequentially. This is useful when tests require significant
//!   resources or interfere with each other when executed in parallel.
//!
//! - `#[env(FOO = "bar")]`
//!   Sets environment variables for a specific test. The harness still
//!   inherits the existing environment, but this allows overriding or adding
//!   variables for individual tests.
//!
//! - `#[exp(nu_experimental::EXAMPLE)]`
//!   Enables a specific [experimental option](nu_experimental) for a test.
//!   It can also be explicitly disabled with
//!   `#[exp(nu_experimental::EXAMPLE = false)]`.
//!
//! Tests with matching environment configurations or experimental settings are grouped together,
//! allowing them to run in parallel where possible.
//!
//! # Writing Integration Tests
//!
//! This crate provides the [`NuTester`](tester::NuTester) struct, which makes it easy to write
//! integration tests that execute Nushell scripts.
//! The main entry point is the [`test()`] function, which returns a `NuTester` instance
//! preconfigured with all commands, relevant environment variables, and the standard library.
//!
//! Each execution group within the test harness receives its own freshly created tester instance.
//! This ensures that environment variables and experimental options are properly isolated between
//! tests.
//!
//! By running tests in-process instead of spawning a separate `nu` binary, tests can be
//! significantly faster.
//! This also improves iteration speed, since the binary does not need to be rebuilt before each
//! run. Additionally, the initial `NuTester` setup is performed once and then cloned, reducing
//! overhead across multiple tests.
//!
//! When writing integration tests, it is recommended to always import the [`prelude`] to avoid
//! repeatedly importing common utilities.
//! Input and output handling relies heavily on the [`IntoValue`](nu_protocol::IntoValue)
//! and [`FromValue`](nu_protocol::FromValue) traits, making it easy to pass data into
//! Nushell and extract values for assertions in a natural way.
//!
//! ## Simple Test Execution and Equality Assertion
//!
//! A basic pattern is to run a Nushell snippet and assert its output:
//!
//! ```
//! # #[macro_use] extern crate nu_test_support;
//! use nu_test_support::prelude::*;
//!
//! #[test]
//! fn short_example() -> Result {
//! #     unimplemented!()
//! # }
//! #
//! # fn main() -> Result {
//!     test()
//!         .run("version | get version")
//!         .expect_value_eq(env!("CARGO_PKG_VERSION"))
//! }
//! ```
//!
//! For improved readability, especially with longer pipelines, it can be
//! helpful to store the script in a variable:
//!
//! ```
//! # #[macro_use] extern crate nu_test_support;
//! use nu_test_support::prelude::*;
//!
//! #[test]
//! fn longer_example() -> Result {
//! #     unimplemented!()
//! # }
//! #
//! # fn main() -> Result {
//!     let code = r#"
//!         [a [b c]]
//!         | flatten
//!         | str join " "
//!     "#;
//!
//!     test().run(code).expect_value_eq("a b c")
//! }
//! ```
//!
//! ## Pulling Data out of Test Run
//!
//! The [`run`](tester::NuTester::run) method of [`NuTester`](tester::NuTester) is commonly used
//! together with [`expect_value_eq`](tester::TestResultExt::expect_value_eq) to compare the
//! result of a script with a value that implements [`IntoValue`](nu_protocol::IntoValue).
//!
//! In cases where direct comparison is not convenient, `run` can also return values by converting
//! them into a type that implements [`FromValue`](nu_protocol::FromValue).
//! This makes it easy to extract data from Nushell and work with it in Rust.
//!
//! ```
//! # #[macro_use] extern crate nu_test_support;
//! use nu_test_support::prelude::*;
//!
//! #[test]
//! fn pull_value_out() -> Result {
//! #     unimplemented!()
//! # }
//! #
//! # fn main() -> Result {
//!     let num: f64 = test().run("12.34 + 2")?;
//!     assert_eq!(num.floor(), 14.0);
//!     Ok(())
//! }
//! ```
//!
//! ## Running Multiple Snippets on a Single Tester
//!
//! Some tests require executing multiple snippets instead of a single pipeline.
//! Running them sequentially can also improve readability, especially for commands that return
//! [`Nothing`](nu_protocol::Value::Nothing).
//!
//! A single tester instance can be reused to execute multiple snippets in order, allowing state to
//! be built up step by step:
//!
//! ```
//! # #[macro_use] extern crate nu_test_support;
//! use nu_test_support::prelude::*;
//!
//! #[test]
//! fn multiple_statements() -> Result {
//! #     unimplemented!()
//! # }
//! #
//! # fn main() -> Result {
//!     let mut tester = test();
//!     let () = tester.run("def parrot [] { '🦜' }")?;
//!     let () = tester.run("def duck [] { '🦆' }")?;
//!     tester
//!         .run("(parrot) + 🤝 + (duck)")
//!         .expect_value_eq("🦜🤝🦆")
//! }
//! ```
//!
//! ## Inserting Data
//!
//! In some cases, it is more convenient to pass data into a pipeline directly
//! instead of constructing it in Nushell code. The
//! [`run_with_data`](tester::NuTester::run_with_data) method supports this by
//! accepting a value that implements [`IntoValue`](nu_protocol::IntoValue).
//!
//! This is also useful to avoid using [`format!`], which can make tests harder
//! to read or reason about.
//!
//! ```
//! # #[macro_use] extern crate nu_test_support;
//! use bytes::Bytes;
//! use nu_test_support::prelude::*;
//!
//! #[test]
//! fn decode_bytes() -> Result {
//! #     unimplemented!()
//! # }
//! #
//! # fn main() -> Result {
//!     test()
//!         .run_with_data("$in | decode", Bytes::from("hello world"))
//!         .expect_value_eq("hello world")
//! }
//! ```
//!
//! Since both [`IntoValue`](nu_protocol::IntoValue) and [`FromValue`](nu_protocol::FromValue) can
//! be derived, custom Rust types can be passed into Nushell and asserted directly.
//! This keeps tests type safe and expressive.
//!
//! ```
//! # #[macro_use] extern crate nu_test_support;
//! use nu_test_support::prelude::*;
//!
//! #[derive(Debug, PartialEq, Eq, Clone, IntoValue, FromValue)]
//! struct Sample {
//!     a: String,
//!     b: u32,
//! }
//!
//! #[test]
//! fn in_and_out() -> Result {
//! #     unimplemented!()
//! # }
//! #
//! # fn main() -> Result {
//!     let sample = Sample {
//!         a: "🐳".into(),
//!         b: 52,
//!     };
//!
//!     test()
//!         .run_with_data("$in | to nuon | from nuon", sample.clone())
//!         .expect_value_eq(sample)
//! }
//! ```
//!
//! ## Working with Metadata or Streams
//!
//! Some tests need access to metadata or streaming data.
//! In these cases, [`run`](tester::NuTester::run) is not sufficient, since it returns a
//! [`Value`](nu_protocol::Value).
//!
//! To work with lower level details, the raw [`PipelineData`](nu_protocol::PipelineData)
//! can be obtained using [`run_raw`](tester::NuTester::run_raw) or
//! [`run_raw_with_data`](tester::NuTester::run_raw_with_data).
//!
//! ```
//! # #[macro_use] extern crate nu_test_support;
//! use nu_test_support::prelude::*;
//!
//! #[test]
//! fn check_metadata() -> Result {
//! #     unimplemented!()
//! # }
//! #
//! # fn main() -> Result {
//!     let mut pipeline_data = test().run_raw("version | to nuon")?.body;
//!     let metadata = pipeline_data.take_metadata().expect("should have metadata");
//!     let content_type = metadata.content_type.expect("should have a content type");
//!     assert_eq!(content_type, "application/x-nuon");
//!     Ok(())
//! }
//! ```
//!
//! ## Configuring the Tester
//!
//! By default, the tester only includes Nushell builtins, the standard library,
//! the `$nu` constant, and a minimal set of environment variables.
//! For example, `$env.PATH` is unset to keep tests deterministic.
//! When needed, the tester can be configured through a set of convenience methods.
//!
//! ### Setting the Working Directory
//!
//! The [`cwd`](tester::NuTester::cwd) method sets the current working directory (`$env.PWD`).
//! This is useful when tests rely on filesystem access relative to a specific location.
//!
//! ```
//! # #[macro_use] extern crate nu_test_support;
//! use nu_test_support::prelude::*;
//!
//! #[test]
//! fn cwd() -> Result {
//! #     unimplemented!()
//! # }
//! #
//! # fn main() -> Result {
//!     test()
//!         .cwd("./crates/nu-test-support")
//!         .run("open Cargo.toml | get package.name")
//!         .expect_value_eq("nu-test-support")
//! }
//! ```
//!
//! ### Configuring the Locale
//!
//! The [`locale`](tester::NuTester::locale) method overrides the locale, while
//! [`locale_en`](tester::NuTester::locale_en) provides a convenient way to force English output.
//! This is helpful when testing locale dependent behavior.
//!
//! ```
//! # #[macro_use] extern crate nu_test_support;
//! use nu_test_support::prelude::*;
//!
//! #[test]
//! fn locale() -> Result {
//! #     unimplemented!()
//! # }
//! #
//! # fn main() -> Result {
//!     let code = r#""2021-10-22 20:00:12 +01:00" | format date "%c""#;
//!     let en: String = test().locale_en().run(&code)?;
//!     let de: String = test().locale("de_DE").run(&code)?;
//!     assert_ne!(en, de);
//!     Ok(())
//! }
//! ```
//!
//! ### Inheriting the System PATH
//!
//! By default, external commands are not available since `$env.PATH` is unset.
//! The [`inherit_path`](tester::NuTester::inherit_path) method restores access to the system PATH,
//! allowing tests to call external binaries.
//!
//! ```
//! # #[macro_use] extern crate nu_test_support;
//! use nu_test_support::prelude::*;
//!
//! #[cfg(windows)]
//! #[test]
//! fn echo() -> Result {
//! #     unimplemented!()
//! # }
//! # #[cfg(windows)]
//! # fn main() -> Result {
//!     test()
//!         .inherit_path()
//!         .run(r#"cmd.exe /c "echo abc""#)
//!         .expect_value_eq("abc")
//! }
//!
//! #[cfg(unix)]
//! #[test]
//! fn echo() -> Result {
//! #     unimplemented!()
//! # }
//! # #[cfg(unix)]
//! # fn main() -> Result {
//!     test()
//!         .inherit_path()
//!         .run(r#"sh -c "echo abc""#)
//!         .expect_value_eq("abc")
//! }
//! ```
//!
//! ### Using the Rust Toolchain
//!
//! The [`inherit_rust_toolchain_env`](tester::NuTester::inherit_rust_toolchain_env)
//! method makes Rust tooling such as `cargo` or `rustc` available inside tests.
//!
//! ```
//! # #[macro_use] extern crate nu_test_support;
//! use nu_test_support::prelude::*;
//!
//! #[test]
//! fn check_cargo_version() -> Result {
//! #     unimplemented!()
//! # }
//! #
//! # fn main() -> Result {
//!     let code = r#"cargo --version | split row " " | get 0"#;
//!     test()
//!         .inherit_rust_toolchain_env()
//!         .run(code)
//!         .expect_value_eq("cargo")
//! }
//! ```
//!
//! ### Running the `nu` Binary
//!
//! The [`add_nu_to_path`](tester::NuTester::add_nu_to_path) method adds the compiled `nu` binary
//! from the `target` directory to the PATH.
//! This allows invoking `nu` itself from within tests.
//! This approach requires rebuilding when behavior changes and should generally be avoided unless
//! necessary.
//!
//! ```
//! # #[macro_use] extern crate nu_test_support;
//! use nu_test_support::prelude::*;
//!
//! #[test]
//! fn cococo() -> Result {
//! #     unimplemented!()
//! # }
//! #
//! # fn main() -> Result {
//!     test()
//!         .add_nu_to_path()
//!         .run("nu --testbin cococo")
//!         .expect_value_eq("cococo")
//! }
//! ```
//!
//! ### Setting Environment Variables
//!
//! The [`env`](tester::NuTester::env) method sets environment variables for the tester itself.
//! Unlike the `#[env]` attribute, this configures the tester instance directly rather than the
//! test harness.
//!
//! ```
//! # #[macro_use] extern crate nu_test_support;
//! use nu_test_support::prelude::*;
//!
//! #[test]
//! fn hey() -> Result {
//! #     unimplemented!()
//! # }
//! #
//! # fn main() -> Result {
//!     test()
//!         .env("HEY", "👋")
//!         .run("$env.HEY")
//!         .expect_value_eq("👋")
//! }
//! ```
//!
//! ## Using the Playground
//!
//! The [`Playground`](playground::Playground) provides a sandboxed filesystem
//! environment for tests. This is especially useful when testing commands
//! that modify the filesystem, such as creating or removing files.
//!
//! Tests typically combine the playground with [`cwd`](tester::NuTester::cwd)
//! to point the tester to the sandboxed directory.
//!
//! ```
//! # #[macro_use] extern crate nu_test_support;
//! use nu_test_support::{fs::Stub::EmptyFile, prelude::*};
//!
//! #[test]
//! fn rm_in_playground() -> Result {
//! #     unimplemented!()
//! # }
//! #
//! # fn main() -> Result {
//!     Playground::setup("rm_in_doctest", |dirs, sandbox| {
//!         sandbox.with_files(&[EmptyFile("i_will_be_deleted.txt")]);
//!         test()
//!             .cwd(dirs.test())
//!             .run("rm i_will_be_deleted.txt")
//!             .expect_value_eq(())
//!     })
//! }
//! ```
//!
//! ## Configuring Experimental Options
//!
//! Experimental features can be enabled or disabled per test using the
//! `#[exp]` attribute provided by the custom test harness.
//!
//! ```no_run
//! # // this is a no_run as we cannot set experimental options safely during a doctest run
//! # #[macro_use] extern crate nu_test_support;
//! use nu_experimental::EXAMPLE;
//! use nu_test_support::prelude::*;
//!
//! #[test]
//! #[exp(EXAMPLE)]
//! fn example_experimental_option() -> Result {
//! #     unimplemented!()
//! # }
//! #
//! # fn main() -> Result {
//!     let code = "debug experimental-options | where identifier == example | get enabled.0";
//!     test().run(code).expect_value_eq(true)
//! }
//! ```
//!
//! ## Using `rstest`
//!
//! The `rstest` crate provides support for fixtures and parameterized test cases, which can
//! significantly reduce boilerplate.
//! It is especially useful when testing the same logic with multiple inputs.
//!
//! It works out of the box with the custom test harness, but requires careful ordering when
//! combined with additional test attributes.
//!
//! ```
//! # #[macro_use] extern crate nu_test_support;
//! use nu_test_support::prelude::*;
//! use rstest::rstest;
//!
//! #[rstest]
//! #[case("a", "a🦜a")]
//! #[case("🦜", "🦜🦜🦜")]
//! fn simple_case(#[case] pre_and_suffix: &str, #[case] result: &str) -> Result {
//! #     unimplemented!()
//! # }
//! #
//! # fn main() -> Result {
//! # let pre_and_suffix = "a";
//! # let result = "a🦜a";
//!     test()
//!         .run_with_data("$in + 🦜 + $in", pre_and_suffix)
//!         .expect_value_eq(result)
//! }
//! ```
//!
//! When combining `rstest` with the custom test harness attributes, the order of attributes
//! becomes important.
//! The harness attribute must be explicitly specified to ensure the test is picked up correctly.
//!
//! ```
//! # #[macro_use] extern crate nu_test_support;
//! use nu_test_support::prelude::*;
//! use rstest::rstest;
//!
//! #[rstest]
//! #[case(1)]
//! #[case(-1)]
//! #[nu_test_support::test]
//! #[env(QUICK_MATHS = "true")]
//! fn math_abs(#[case] input: i32) -> Result {
//! #     unimplemented!()
//! # }
//! #
//! # fn main() -> Result {
//! # let input: i32 = 1;
//!     test()
//!         .run_with_data("$in | math abs", input)
//!         .expect_value_eq(1)
//! }
//! ```

pub mod assertions;
pub mod fs;
pub mod harness;
pub mod net;
pub mod playground;

pub mod deprecated;
#[doc(no_inline)]
pub use deprecated::*;

pub mod tester;
pub use tester::{Result, ShellErrorExt, TestError as Error, TestResultExt, test};

/// Prelude for writing tests.
pub mod prelude {
    #[doc(no_inline)]
    pub use super::{
        Outcome,
        assertions::*,
        nu,
        playground::Playground,
        tester::{Result, ShellErrorExt, TestError as Error, TestResultExt, test},
    };

    #[doc(no_inline)]
    pub use nu_protocol::{CompileError, FromValue, IntoValue, ParseError, ShellError, Value};
}

// Expose macros to be used for the test harness.
pub use harness::macros::*;

// Needs to be reexported for `nu!` macro
pub use nu_path;

// Export json macro to allow writing json values easily.
#[doc(no_inline)]
pub use serde_json::json;

/// Build a [`CellPath`](nu_protocol::ast::CellPath) in Rust using the familiar cell path syntax.
///
/// This macro lets you write cell paths the same way you do in Nushell.
/// It also supports inline variables or expressions by wrapping them in a group (parentheses).
///
/// # Examples
///
/// ```rust
/// use nu_test_support::test_cell_path;
///
/// let simple = test_cell_path!(foo.bar);
/// assert_eq!(simple.to_string(), "$.foo.bar");
///
/// let with_modifiers = test_cell_path!(foo?.bar!);
/// assert_eq!(with_modifiers.to_string(), "$.foo?.bar!");
///
/// let with_literal = test_cell_path!(foo."bar baz".3);
/// assert_eq!(with_literal.to_string(), r#"$.foo."bar baz".3"#);
///
/// let column = "foo";
/// let index = 3;
/// let from_vars = test_cell_path!((column).(index));
/// assert_eq!(from_vars.to_string(), "$.foo.3");
/// ```
#[doc(inline)]
pub use nu_test_support_macros::test_cell_path;
