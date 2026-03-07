use std::{env, path::PathBuf, sync::LazyLock};

use nu_protocol::{
    CompileError, FromValue, IntoValue, ParseError, PipelineData, PipelineExecutionData,
    ShellError, Span, Value,
    debugger::WithoutDebug,
    engine::{EngineState, Stack, StateWorkingSet},
};
use nu_utils::sync::KeyedLazyLock;
use thiserror::Error;

use crate::harness::group::GroupKey;

static ROOT: LazyLock<PathBuf> = LazyLock::new(|| {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("could not canonicalize root")
});

// By using different engine states depending on the group key, we can ensure that behavior from
// experimental options or environment variables take proper effect in the setup of an engine state.
static INITIAL_ENGINE_STATES: KeyedLazyLock<GroupKey, EngineState> = KeyedLazyLock::new(|_| {
    let engine_state = nu_cmd_lang::create_default_context();
    let engine_state = nu_command::add_shell_command_context(engine_state);
    let mut engine_state = nu_cmd_extra::add_extra_command_context(engine_state);

    engine_state.generate_nu_constant();
    engine_state.add_env_var("PWD".into(), Value::test_string(ROOT.to_string_lossy()));
    nu_std::load_standard_library(&mut engine_state).expect("could not load standard library");

    engine_state
});

/// Create a [`NuTester`] for running Nushell snippets in tests.
///
/// Prefer this helper over the `nu!` macro for most tests.
/// It runs snippets in-process instead of shelling out to a subprocess, which makes tests faster
/// and lets you pass and read values directly without inferring from stdout or stderr.
/// The `nu!` macro executes the `nu` binary, and changes in a single crate might not trigger a
/// rebuild of that binary, so tests can run against stale behavior unless you run `cargo build`
/// first.
/// Using this helper avoids that by executing against the in-process engine components.
///
/// The tester starts from a default [`EngineState`] with the standard library loaded, and a fresh
/// [`Stack`].
/// Use the returned value to configure environment variables or the working directory before
/// running code.
///
/// # Environment behavior
///
/// - This tester does not inherit process environment variables.
/// - Any variables you want available to the engine must be added explicitly via
///   [`NuTester::env`] (or convenience helpers like [`NuTester::locale`]).
/// - Experimental options and other external environment settings are respected
///   when constructing the underlying engine state for the current test group.
///
/// # Examples
///
/// ```rust
/// use nu_test_support::prelude::*;
///
/// let code = "use std/util ellie; ellie | ansi strip";
/// let value: String = test().run(code)?;
/// assert_eq!(value, r#"
///      __  ,
///  .--()°'.'
/// '|, . ,'
///  !_-(_\
/// "#.trim_matches('\n'));
/// # Ok::<(), nu_test_support::tester::TestError>(())
/// ```
///
/// ```rust
/// use nu_test_support::prelude::*;
///
/// let mut tester = test()
///     .env("FOO", "bar")
///     .cwd("crates/nu-test-support");
///
/// let value: String = tester.run("$env.FOO")?;
/// assert_eq!(value, "bar");
/// # Ok::<(), nu_test_support::tester::TestError>(())
/// ```
pub fn test() -> NuTester {
    NuTester::default()
}

/// Helper for running Nushell code in tests.
///
/// `NuTester` owns an [`EngineState`] and [`Stack`] that are reused across invocations.
/// Configuration methods update the engine state before execution.
#[derive(Clone)]
pub struct NuTester {
    engine_state: EngineState,
    stack: Stack,
}

impl Default for NuTester {
    /// Create a default tester.
    ///
    /// Prefer [`test()`] for a shorter entry point that avoids naming [`NuTester`].
    fn default() -> Self {
        Self {
            engine_state: INITIAL_ENGINE_STATES.get(&GroupKey::current()).clone(),
            stack: Stack::new(),
        }
    }
}

impl NuTester {
    /// Create a default tester with the standard engine state.
    ///
    /// Prefer [`test()`] for a shorter entry point that avoids naming [`NuTester`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the working directory used for evaluation.
    ///
    /// Relative paths are resolved from the repository root and canonicalized.
    pub fn cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        let cwd = cwd.into();

        let cwd = match cwd.is_absolute() {
            true => cwd,
            false => ROOT
                .join(cwd)
                .canonicalize()
                .expect("could not canonicalize path"),
        };

        self.engine_state
            .add_env_var("PWD".into(), Value::test_string(cwd.to_string_lossy()));
        self
    }

    /// Set the locale used by tests via `NU_TEST_LOCALE`.
    pub fn locale(mut self, locale: impl Into<String>) -> Self {
        self.engine_state
            .add_env_var("NU_TEST_LOCALE".into(), Value::test_string(locale.into()));
        self
    }

    /// Set the locale to `en_US.utf8`.
    pub fn locale_en(self) -> Self {
        self.locale("en_US.utf8")
    }

    /// Add a custom environment variable to the engine state.
    pub fn env(mut self, key: impl Into<String>, val: impl Into<String>) -> Self {
        self.engine_state
            .add_env_var(key.into(), Value::test_string(val.into()));
        self
    }

    /// Run Nushell code and extract the value into `T`.
    ///
    /// Parsing, compilation, or evaluation failures are returned as [`TestError`].
    pub fn run<T: FromValue>(&mut self, code: impl AsRef<str>) -> Result<T, TestError> {
        Self::extract_value(self.run_raw(code)?)
    }

    /// Run Nushell code with input data and extract the value into `T`.
    ///
    /// The input value is converted into `PipelineData` using [`IntoValue`].
    pub fn run_with_data<T: FromValue>(
        &mut self,
        code: impl AsRef<str>,
        data: impl IntoValue,
    ) -> Result<T, TestError> {
        let input = PipelineData::value(data.into_value(Span::test_data()), None);
        Self::extract_value(self.run_raw_with_data(code, input)?)
    }

    /// Run Nushell code and return the raw [`PipelineExecutionData`].
    pub fn run_raw(&mut self, code: impl AsRef<str>) -> Result<PipelineExecutionData, TestError> {
        self.run_raw_with_data(code, PipelineData::empty())
    }

    /// Run Nushell code with input data and return the raw execution results.
    ///
    /// This parses, compiles, and evaluates the code against the current engine state.
    pub fn run_raw_with_data(
        &mut self,
        code: impl AsRef<str>,
        data: PipelineData,
    ) -> Result<PipelineExecutionData, TestError> {
        let code = code.as_ref().as_bytes();

        let mut working_set = StateWorkingSet::new(&self.engine_state);
        let block = nu_parser::parse(&mut working_set, None, code, false);

        if let Some(err) = working_set.parse_errors.into_iter().next() {
            return Err(err.into());
        }

        if let Some(err) = working_set.compile_errors.into_iter().next() {
            return Err(err.into());
        }

        self.engine_state.merge_delta(working_set.delta)?;
        nu_engine::eval_block::<WithoutDebug>(&self.engine_state, &mut self.stack, &block, data)
            .map_err(Into::into)
    }

    fn extract_value<T: FromValue>(
        pipeline_execution_data: PipelineExecutionData,
    ) -> Result<T, TestError> {
        let pipeline_data = pipeline_execution_data.body;
        let value = pipeline_data.into_value(Span::test_data())?;
        let value = T::from_value(value)?;
        Ok(value)
    }
}

/// Errors emitted by `NuTester` when parsing, compiling, or evaluating code.
///
/// This enum is marked as non-exhaustive to allow adding new variants.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum TestError {
    #[error(transparent)]
    Parse(#[from] ParseError),

    #[error(transparent)]
    Compile(#[from] CompileError),

    #[error(transparent)]
    Shell(#[from] ShellError),

    #[error("got no error")]
    None,

    #[error("expected an inner error value but got none")]
    NoInner,

    #[error("the error is not a generic shell error")]
    NotGeneric,
}

impl TestError {
    /// Convert this error into a [`ParseError`], if it is one.
    pub fn parse(self) -> Result<ParseError, TestError> {
        match self {
            Self::Parse(err) => Ok(err),
            err => Err(err),
        }
    }

    /// Convert this error into a [`CompileError`], if it is one.
    pub fn compile(self) -> Result<CompileError, TestError> {
        match self {
            Self::Compile(err) => Ok(err),
            err => Err(err),
        }
    }

    /// Convert this error into a [`ShellError`], if it is one.
    pub fn shell(self) -> Result<ShellError, TestError> {
        match self {
            Self::Shell(err) => Ok(err),
            err => Err(err),
        }
    }
}

/// Convenience result type for test helpers.
pub type Result<T = (), E = TestError> = std::result::Result<T, E>;

/// Extensions for asserting error kinds from test helpers.
pub trait TestResultExt: Sized {
    /// Expect the result to be a [`ShellError`].
    fn expect_shell_error(self) -> Result<ShellError, TestError>;
    /// Expect the result to be a [`ParseError`].
    fn expect_parse_error(self) -> Result<ParseError, TestError>;
    /// Expect the result to be a [`CompileError`].
    fn expect_compile_error(self) -> Result<CompileError, TestError>;

    /// Expect the result to be a [`ShellError`].
    fn expect_error(self) -> Result<ShellError, TestError> {
        self.expect_shell_error()
    }
}

impl TestResultExt for Result {
    fn expect_shell_error(self) -> Result<ShellError, TestError> {
        match self {
            Ok(()) => Err(TestError::None),
            Err(TestError::Shell(err)) => Ok(err),
            Err(err) => Err(err),
        }
    }

    fn expect_parse_error(self) -> Result<ParseError, TestError> {
        match self {
            Ok(()) => Err(TestError::None),
            Err(TestError::Parse(err)) => Ok(err),
            Err(err) => Err(err),
        }
    }

    fn expect_compile_error(self) -> Result<CompileError, TestError> {
        match self {
            Ok(()) => Err(TestError::None),
            Err(TestError::Compile(err)) => Ok(err),
            Err(err) => Err(err),
        }
    }
}

/// Extensions for interrogating [`ShellError`] values in tests.
pub trait ShellErrorExt {
    /// Tries to convert into an inner value from a [`ShellError`].
    ///
    /// Useful if the error is expected to be a generic error that contains an inner error or a
    /// chained error that chained another error.
    ///
    /// However, this function returns [`None`]
    /// - if `inner` of [`ShellError::GenericError`] is empty
    /// - if `sources` of [`ShellError::ChainedError`] is empty
    /// - the error is none of the above types
    ///
    /// So make sure that a [`None`] value is not surprise.
    fn into_inner(self) -> Result<ShellError, TestError>;

    /// Extract the error field from [`ShellError::GenericError`], if it is one.
    fn generic_error(self) -> Result<String, TestError>;

    /// Extract the message field from [`ShellError::GenericError`], if it is one.
    fn generic_msg(self) -> Result<String, TestError>;
}

impl ShellErrorExt for ShellError {
    fn into_inner(self) -> Result<ShellError, TestError> {
        match self {
            ShellError::GenericError { inner, .. } => {
                inner.into_iter().next().ok_or(TestError::NoInner)
            }
            ShellError::ChainedError(err) => err.sources_iter().next().ok_or(TestError::NoInner),
            _ => Err(TestError::NoInner),
        }
    }

    fn generic_error(self) -> Result<String, TestError> {
        match self {
            ShellError::GenericError { error, .. } => Ok(error),
            _ => Err(TestError::NotGeneric),
        }
    }

    fn generic_msg(self) -> Result<String, TestError> {
        match self {
            ShellError::GenericError { msg, .. } => Ok(msg),
            _ => Err(TestError::NotGeneric),
        }
    }
}
