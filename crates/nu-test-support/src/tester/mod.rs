use std::{env, path::PathBuf, sync::LazyLock};

use nu_protocol::{
    CompileError, FromValue, IntoValue, ParseError, PipelineData, PipelineExecutionData,
    ShellError, Span, Value,
    debugger::WithoutDebug,
    engine::{EngineState, Stack, StateWorkingSet},
};
use thiserror::Error;

static ROOT: LazyLock<PathBuf> = LazyLock::new(|| {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("could not canonicalize root")
});

static INITIAL_ENGINE_STATE: LazyLock<EngineState> = LazyLock::new(|| {
    let engine_state = nu_cmd_lang::create_default_context();
    let engine_state = nu_command::add_shell_command_context(engine_state);
    let mut engine_state = nu_cmd_extra::add_extra_command_context(engine_state);

    engine_state.generate_nu_constant();
    engine_state.add_env_var("PWD".into(), Value::test_string(ROOT.to_string_lossy()));
    nu_std::load_standard_library(&mut engine_state).expect("could not load standard library");

    engine_state
});

pub fn test() -> NuTester {
    NuTester::default()
}

#[derive(Clone)]
pub struct NuTester {
    engine_state: EngineState,
    stack: Stack,
}

impl Default for NuTester {
    fn default() -> Self {
        Self {
            engine_state: INITIAL_ENGINE_STATE.clone(),
            stack: Stack::new(),
        }
    }
}

impl NuTester {
    pub fn new() -> Self {
        Self::default()
    }

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

    pub fn locale(mut self, locale: impl Into<String>) -> Self {
        self.engine_state
            .add_env_var("NU_TEST_LOCALE".into(), Value::test_string(locale.into()));
        self
    }

    pub fn locale_en(self) -> Self {
        self.locale("en_US.utf8")
    }

    pub fn env(mut self, key: impl Into<String>, val: impl Into<String>) -> Self {
        self.engine_state
            .add_env_var(key.into(), Value::test_string(val.into()));
        self
    }

    pub fn run<T: FromValue>(&mut self, code: impl AsRef<str>) -> Result<T, TestError> {
        Self::extract_value(self.run_raw(code)?)
    }

    pub fn run_with_data<T: FromValue>(
        &mut self,
        code: impl AsRef<str>,
        data: impl IntoValue,
    ) -> Result<T, TestError> {
        let input = PipelineData::value(data.into_value(Span::test_data()), None);
        Self::extract_value(self.run_raw_with_data(code, input)?)
    }

    pub fn run_raw(&mut self, code: impl AsRef<str>) -> Result<PipelineExecutionData, TestError> {
        self.run_raw_with_data(code, PipelineData::empty())
    }

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
    pub fn parse(self) -> Result<ParseError, TestError> {
        match self {
            Self::Parse(err) => Ok(err),
            err => Err(err),
        }
    }

    pub fn compile(self) -> Result<CompileError, TestError> {
        match self {
            Self::Compile(err) => Ok(err),
            err => Err(err),
        }
    }

    pub fn shell(self) -> Result<ShellError, TestError> {
        match self {
            Self::Shell(err) => Ok(err),
            err => Err(err),
        }
    }
}

pub type Result<T = (), E = TestError> = std::result::Result<T, E>;

pub trait TestResultExt: Sized {
    fn expect_shell_error(self) -> Result<ShellError, TestError>;
    fn expect_parse_error(self) -> Result<ParseError, TestError>;
    fn expect_compile_error(self) -> Result<CompileError, TestError>;

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

    fn generic_error(self) -> Result<String, TestError>;

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
