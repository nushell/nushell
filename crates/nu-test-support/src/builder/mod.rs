use std::{env, path::PathBuf, sync::LazyLock};

use nu_protocol::{
    CompileError, FromValue, ParseError, PipelineData, PipelineExecutionData, ShellError, Span,
    Value,
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

    pub fn run_raw(&mut self, code: impl AsRef<str>) -> Result<PipelineExecutionData, TestError> {
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
        nu_engine::eval_block::<WithoutDebug>(
            &self.engine_state,
            &mut self.stack,
            &block,
            PipelineData::empty(),
        )
        .map_err(Into::into)
    }

    pub fn run<T: FromValue>(&mut self, code: impl AsRef<str>) -> Result<T, TestError> {
        let pipeline_data = self.run_raw(code)?.body;
        let value = pipeline_data.into_value(Span::test_data())?;
        let value = T::from_value(value)?;
        Ok(value)
    }
}

#[non_exhaustive] // motivate test implementors to use provided methods
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
