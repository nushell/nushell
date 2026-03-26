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
//! By running tests in process instead of spawning a separate `nu` binary, tests can be
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
pub use serde_json::json;
