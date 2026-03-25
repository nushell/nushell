//! Test support for the nushell crates.
//!
//! ```
//! # use nu_test_support::harness::macros::test;
//! use nu_test_support::prelude::*;
//!
//! #[test]
//! #[serial]
//! fn a() -> Result {
//!     # unimplemented!()
//! # }
//! #
//! # fn main() -> Result {
//!     test().run("version | get version").expect_value_eq(env!("CARGO_PKG_VERSION"))
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
