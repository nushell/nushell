use std::{
    env,
    error::Error,
    fmt::{Debug, Display},
    panic::Location,
    path::{Path, PathBuf},
    sync::{Arc, LazyLock},
};

use miette::Diagnostic;
use nu_protocol::{
    CompileError, Config, FromValue, IntoValue, LabeledError, ParseError, PipelineData,
    PipelineExecutionData, ShellError, Span, Value,
    ast::Block,
    debugger::WithoutDebug,
    engine::{Command, EngineState, Stack, StateDelta, StateWorkingSet},
    shell_error::{io::IoError, network::NetworkError},
};
use nu_utils::{consts::ENV_PATH_SEPARATOR_CHAR, sync::KeyedLazyLock};
use parking_lot::{RwLock, const_rwlock};

use crate::harness::group::GroupKey;

#[cfg(feature = "plugin")]
use nu_plugin_engine::{GetPlugin, PersistentPlugin, PluginDeclaration};
#[cfg(feature = "plugin")]
use nu_protocol::{PluginIdentity, PluginSignature, RegisteredPlugin};

static ROOT: LazyLock<PathBuf> = LazyLock::new(|| {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("could not canonicalize root")
});

// By using different engine states depending on the group key, we can ensure that behavior from
// experimental options or environment variables take proper effect in the setup of an engine state.
static INITIAL_ENGINE_STATES: KeyedLazyLock<GroupKey, EngineState> = KeyedLazyLock::new(|_| {
    // Some modules below are commented out because they don't depend on nu-test-support
    // Copied from `nu::command_context::add_command_context`
    let engine_state = nu_cmd_lang::create_default_context();
    #[cfg(feature = "plugin")]
    let engine_state = nu_cmd_plugin::add_plugin_command_context(engine_state);
    let engine_state = nu_command::add_shell_command_context(engine_state);
    let engine_state = nu_cmd_extra::add_extra_command_context(engine_state);
    #[cfg(feature = "os")]
    let engine_state = nu_cli::add_cli_context(engine_state);
    // let engine_state = nu_explore::add_explore_context(engine_state);

    // Make `engine_state` mutable without fiddling with features
    let mut engine_state = engine_state;

    engine_state.generate_nu_constant();
    [
        ("PWD", Value::test_string(ROOT.to_string_lossy())),
        ("config", Config::default().into_value(Span::unknown())),
        ("NO_COLOR", Value::test_bool(true)),
    ]
    .into_iter()
    .for_each(|(key, val)| engine_state.add_env_var(key.into(), val));

    // Should this be inherited or do we want tighter testing?
    #[cfg(windows)]
    if let Ok(path_ext) = env::var("PATHEXT") {
        engine_state.add_env_var("PATHEXT".into(), Value::test_string(path_ext));
    }

    nu_std::load_standard_library(&mut engine_state).expect("could not load standard library");

    engine_state
});

/// Plugin auto loader for [`THREAD_PLUGIN_AUTO_LOAD`] and [`GLOBAL_PLUGIN_AUTO_LOAD`].
#[cfg(feature = "plugin")]
#[derive(Debug, Clone)]
pub struct PluginAutoLoader {
    pub identity: Arc<PluginIdentity>,
    pub plugin: Option<Arc<PersistentPlugin>>,
    pub signatures: Option<Arc<[PluginSignature]>>,
}

/// Paths to be loaded into the PATH env variable of a [`NuTester`].
pub static PATH_ENV_AUTO_LOAD: RwLock<Vec<PathBuf>> = const_rwlock(Vec::new());

/// Plugins to be automatically loaded into a [`NuTester`].
#[cfg(feature = "plugin")]
pub static PLUGIN_AUTO_LOAD: RwLock<Vec<PluginAutoLoader>> = const_rwlock(Vec::new());

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
/// # Auto loaders
///
/// The `*_AUTO_LOAD` statics automatically prepare the test [`EngineState`].
///
/// [`PATH_ENV_AUTO_LOAD`] loads paths into the tester's `PATH` environment variable, allowing
/// binaries to be found without adding them manually in each [`test()`].
///
/// When the `plugin` feature is enabled,
#[cfg_attr(feature = "plugin", doc = "[`PLUGIN_AUTO_LOAD`]")]
#[cfg_attr(not(feature = "plugin"), doc = "`PLUGIN_AUTO_LOAD`")]
/// loads plugins into the tester so they can be called during tests.
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
    let tester = NuTester {
        engine_state: INITIAL_ENGINE_STATES.get(&GroupKey::current()).clone(),
        stack: Stack::new().collect_value(),
        fname_counter: Counter::default(),
    };

    let tester = tester.append_path(&*PATH_ENV_AUTO_LOAD.read());

    #[cfg(feature = "plugin")]
    let tester = tester.auto_load_plugins();

    tester
}

/// Helper for running Nushell code in tests.
///
/// `NuTester` owns an [`EngineState`] and [`Stack`] that are reused across invocations.
/// Configuration methods update the engine state before execution.
#[derive(Clone)]
#[non_exhaustive] // Ensure this type is only generated using `test()`, `new()` or `default()`.
pub struct NuTester {
    pub engine_state: EngineState,
    pub stack: Stack,

    /// Counter that is used for parsing source code with different "file names".
    fname_counter: Counter,
}

#[derive(Default, Clone)]
struct Counter(u64);

impl Counter {
    pub fn get(&mut self) -> u64 {
        let value = self.0;
        self.0 += 1;
        value
    }
}

impl Default for NuTester {
    /// Create a default tester.
    ///
    /// Prefer [`test()`] for a shorter entry point that avoids naming [`NuTester`].
    fn default() -> Self {
        test()
    }
}

#[cfg(feature = "plugin")]
impl NuTester {
    /// Load the plugins from [`PLUGIN_AUTO_LOAD`] into the [`NuTester`].
    ///
    /// Called in [`test`], do not call somewhere else again.
    fn auto_load_plugins(self) -> Self {
        let mut tester = self;
        let auto_loaders = PLUGIN_AUTO_LOAD.read();
        if auto_loaders.is_empty() {
            return tester;
        }

        let mut working_set = StateWorkingSet::new(&tester.engine_state);
        for auto_loader in auto_loaders.iter() {
            let plugin = working_set.find_or_create_plugin(&auto_loader.identity, || {
                auto_loader
                    .plugin
                    .as_ref()
                    .map(|plugin| plugin.clone())
                    .unwrap_or_else(|| {
                        Arc::new(PersistentPlugin::new(
                            (*auto_loader.identity).clone(),
                            Default::default(),
                        ))
                    })
            });

            let plugin: Arc<PersistentPlugin> = plugin
                .as_any()
                .downcast()
                .expect("could not downcast to persistent plugin");

            // if preloaded by our test harness, we don't need to construct a plugin
            // interface here
            let mut interface = None;

            // our test harness also sets metadata, so we don't have to do again
            if plugin.metadata().is_none() {
                let interface = interface.get_or_insert_with(|| {
                    plugin
                        .clone()
                        .get_plugin(None)
                        .expect("could not get plugin")
                });

                plugin.set_metadata(Some(
                    interface
                        .get_metadata()
                        .expect("could not get plugin metadata"),
                ));
            }

            // our test harness also preloads signatures, assuming they don't change
            let signatures = auto_loader
                .signatures
                .as_deref()
                .map(|signatures| signatures.to_owned())
                .unwrap_or_else(|| {
                    let interface = interface.get_or_insert_with(|| {
                        plugin
                            .clone()
                            .get_plugin(None)
                            .expect("could not get plugin")
                    });
                    interface
                        .get_signature()
                        .expect("could not get plugin signatures")
                });

            for signature in signatures {
                let decl = PluginDeclaration::new(plugin.clone(), signature);
                working_set.add_decl(Box::new(decl));
            }
        }

        tester
            .engine_state
            .merge_delta(working_set.render())
            .expect("could not merge plugin working set");

        tester
    }
}

impl NuTester {
    /// Create a default tester with the standard engine state.
    ///
    /// Prefer [`test()`] for a shorter entry point that avoids naming [`NuTester`].
    pub fn new() -> Self {
        test()
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

    /// Set the locale used by tests via `NU_TEST_LOCALE_OVERRIDE`.
    pub fn locale(mut self, locale: impl Into<String>) -> Self {
        self.engine_state.add_env_var(
            "NU_TEST_LOCALE_OVERRIDE".into(),
            Value::test_string(locale.into()),
        );
        self
    }

    /// Set the locale to `en_US.utf8`.
    pub fn locale_en(self) -> Self {
        self.locale("en_US.utf8")
    }

    /// Get the current path env.
    fn path(&self) -> Vec<Value> {
        match self.engine_state.get_env_var("PATH") {
            None => Vec::new(),
            Some(Value::List { vals, .. }) => vals.to_vec(),
            Some(Value::String { val, .. }) => val
                .split(ENV_PATH_SEPARATOR_CHAR)
                .map(Value::test_string)
                .collect(),
            Some(v) => panic!("PATH is neither a list nor a string, is {}", v.get_type()),
        }
    }

    /// Prepend entries to the PATH.
    pub fn prepend_path(self, entries: impl IntoIterator<Item = impl AsRef<Path>>) -> Self {
        let path = entries
            .into_iter()
            .map(|item| Value::test_string(item.as_ref().to_string_lossy()))
            .chain(self.path())
            .collect();
        self.env("PATH", Value::test_list(path))
    }

    /// Append entries to the PATH.
    pub fn append_path(self, entries: impl IntoIterator<Item = impl AsRef<Path>>) -> Self {
        let path = self
            .path()
            .into_iter()
            .chain(
                entries
                    .into_iter()
                    .map(|item| Value::test_string(item.as_ref().to_string_lossy())),
            )
            .collect();
        self.env("PATH", Value::test_list(path))
    }

    /// Inherit the `PATH` environment variable from the running process by appending it.
    ///
    /// This is useful for tests that spawn external commands and should resolve
    /// binaries the same way as the parent test process.
    ///
    /// Panics if `PATH` is not set in the current process environment.
    pub fn inherit_path(self) -> Self {
        let path = env::var("PATH").expect("PATH not available in env");
        self.append_path(path.split(ENV_PATH_SEPARATOR_CHAR))
    }

    /// Inherit an environment variable from the running process, but only if it is set.
    ///
    /// This is useful for optional variables whose absence should not cause a panic.
    pub fn inherit_env_if_set(self, key: impl AsRef<str>) -> Self {
        let key = key.as_ref();
        match env::var(key) {
            Ok(val) => self.env(key, val),
            Err(_) => self,
        }
    }

    /// Inherit Rust toolchain related environment variables from the running process,
    /// but only when they are set.
    ///
    /// This helps tests that spawn `cargo`, `rustc`, or `rustup` behave more like
    /// the parent process, especially when the active toolchain or install location
    /// is configured through environment variables.
    ///
    /// The following variables are inherited when present:
    /// - `PATH`
    /// - `CARGO_HOME`
    /// - `RUSTUP_HOME`
    /// - `RUSTUP_TOOLCHAIN`
    /// - `RUSTUP_DIST_SERVER`
    /// - `RUSTUP_UPDATE_ROOT`
    ///
    /// Proxy variables are also inherited when present since `rustup` may need them
    /// to download or resolve toolchain metadata:
    /// - `HTTP_PROXY`, `HTTPS_PROXY`, `NO_PROXY`
    /// - `http_proxy`, `https_proxy`, `no_proxy`
    ///
    /// This does not guarantee identical behavior to an interactive shell since the
    /// current working directory can still affect rustup toolchain resolution.
    pub fn inherit_rust_toolchain_env(self) -> Self {
        self.inherit_path()
            .inherit_env_if_set("PATH")
            .inherit_env_if_set("CARGO_HOME")
            .inherit_env_if_set("RUSTUP_HOME")
            .inherit_env_if_set("RUSTUP_TOOLCHAIN")
            .inherit_env_if_set("RUSTUP_DIST_SERVER")
            .inherit_env_if_set("RUSTUP_UPDATE_ROOT")
            .inherit_env_if_set("HTTP_PROXY")
            .inherit_env_if_set("HTTPS_PROXY")
            .inherit_env_if_set("NO_PROXY")
            .inherit_env_if_set("http_proxy")
            .inherit_env_if_set("https_proxy")
            .inherit_env_if_set("no_proxy")
    }

    /// Adds the "nu" binary for testing to the path.
    ///
    /// Calling [`inherit_path`](Self::inherit_path) after this methods removes the path entry.
    #[deprecated(note = "use `#[deps(NU)]` instead")]
    pub fn add_nu_to_path(self) -> Self {
        let nu_home = crate::fs::binaries();
        let path = self.engine_state.get_env_var("PATH");
        let path = match path {
            None => nu_home.display().to_string(),
            Some(path) => format!(
                "{nu}{sep}{prev}",
                nu = nu_home.display(),
                sep = ENV_PATH_SEPARATOR_CHAR,
                prev = path.as_str().expect("PATH should always be a string")
            ),
        };
        self.env("PATH", path)
    }

    /// Add a custom environment variable to the engine state.
    pub fn env(mut self, key: impl Into<String>, val: impl IntoValue) -> Self {
        self.engine_state
            .add_env_var(key.into(), val.into_value(Span::test_data()));
        self
    }

    /// Run Nushell code and extract the value into `T`.
    ///
    /// Parsing, compilation, or evaluation failures are returned as [`TestError`].
    #[track_caller]
    pub fn run<T: FromValue>(&mut self, code: impl AsRef<str>) -> Result<T> {
        Self::extract_value(self.run_raw(code)?)
    }

    /// Run Nushell code with input data and extract the value into `T`.
    ///
    /// The input value is converted into `PipelineData` using [`IntoValue`].
    #[track_caller]
    pub fn run_with_data<T: FromValue>(
        &mut self,
        code: impl AsRef<str>,
        data: impl IntoValue,
    ) -> Result<T> {
        let input = PipelineData::value(data.into_value(Span::test_data()), None);
        Self::extract_value(self.run_raw_with_data(code, input)?)
    }

    /// Run Nushell code and return the raw [`PipelineExecutionData`].
    #[track_caller]
    pub fn run_raw(&mut self, code: impl AsRef<str>) -> Result<PipelineExecutionData> {
        self.run_raw_with_data(code, PipelineData::empty())
    }

    /// Run Nushell code with input data and return the raw execution results.
    ///
    /// This parses, compiles, and evaluates the code against the current engine state.
    #[track_caller]
    pub fn run_raw_with_data(
        &mut self,
        code: impl AsRef<str>,
        data: PipelineData,
    ) -> Result<PipelineExecutionData> {
        let location = TestLocation(Location::caller());
        let (delta, block) = self.parse_and_compile(code)?;
        self.engine_state.merge_delta(delta)?;
        nu_engine::eval_block::<WithoutDebug>(&self.engine_state, &mut self.stack, &block, data)
            .map_err(|err| TestError {
                location,
                kind: TestErrorKind::Shell(err),
            })
    }

    #[track_caller]
    pub fn parse_and_compile(&mut self, code: impl AsRef<str>) -> Result<(StateDelta, Arc<Block>)> {
        let location = TestLocation(Location::caller());
        let code = code.as_ref().as_bytes();

        let mut working_set = StateWorkingSet::new(&self.engine_state);
        let fname = format!("nu-tester-{}", self.fname_counter.get());
        let block = nu_parser::parse(&mut working_set, Some(&fname), code, false);

        if let Some(err) = working_set.parse_errors.into_iter().next() {
            return Err(TestError {
                location,
                kind: TestErrorKind::Parse(err),
            });
        }

        if let Some(err) = working_set.compile_errors.into_iter().next() {
            return Err(TestError {
                location,
                kind: TestErrorKind::Compile(err),
            });
        }

        Ok((working_set.delta, block))
    }

    #[track_caller]
    fn extract_value<T: FromValue>(
        pipeline_execution_data: PipelineExecutionData,
    ) -> Result<T, TestError> {
        let pipeline_data = pipeline_execution_data.body;
        let value = pipeline_data.into_value(Span::test_data())?;
        let value = T::from_value(value)?;
        Ok(value)
    }

    /// Test examples of a command.
    #[track_caller]
    pub fn examples(&mut self, command: impl Command + 'static) -> Result {
        let location = TestLocation(Location::caller());
        for example in command.examples() {
            match example.result {
                None => self
                    .parse_and_compile(example.example)
                    .map(|_| ())
                    .map_err(|err| TestError {
                        location,
                        kind: TestErrorKind::ExampleFailed {
                            command: command.name().to_string(),
                            description: example.description.to_string(),
                            code: example.example.to_string(),
                            err: Box::new(err.kind),
                        },
                    })?,
                Some(expected) => {
                    let got = self.clone().run(example.example)?;
                    if got != expected {
                        return Err(TestError {
                            location,
                            kind: TestErrorKind::ExampleFailed {
                                command: command.name().to_string(),
                                description: example.description.to_string(),
                                code: example.example.to_string(),
                                err: Box::new(TestErrorKind::UnexpectedValue { expected, got }),
                            },
                        });
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TestError {
    location: TestLocation,
    kind: TestErrorKind,
}

#[derive(Clone, Copy, PartialEq, derive_more::Debug)]
#[debug("{_0}")]
pub struct TestLocation(&'static Location<'static>);

/// Errors emitted by `NuTester` when parsing, compiling, or evaluating code.
///
/// This enum is marked as non-exhaustive to allow adding new variants.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum TestErrorKind {
    Parse(ParseError),
    Compile(CompileError),
    Shell(ShellError),
    GotValue {
        got: Value,
    },
    NoInner,
    UnexpectedErrorKind {
        expected: &'static str,
        got: ShellError,
    },
    UnexpectedValue {
        expected: Value,
        got: Value,
    },
    NoCode {
        expected: String,
    },
    UnexpectedCode {
        expected: String,
        got: String,
    },
    ExampleFailed {
        command: String,
        description: String,
        code: String,
        err: Box<TestErrorKind>,
    },
}

impl Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:#?}")
    }
}

impl Error for TestError {}

impl From<ShellError> for TestError {
    #[track_caller]
    fn from(err: ShellError) -> Self {
        Self {
            location: TestLocation(Location::caller()),
            kind: TestErrorKind::Shell(err),
        }
    }
}

impl From<ParseError> for TestError {
    #[track_caller]
    fn from(err: ParseError) -> Self {
        Self {
            location: TestLocation(Location::caller()),
            kind: TestErrorKind::Parse(err),
        }
    }
}

impl TestError {
    /// Convert this error into a [`ParseError`], if it is one.
    pub fn parse(self) -> Result<ParseError, TestError> {
        match self.kind {
            TestErrorKind::Parse(err) => Ok(err),
            _ => Err(self),
        }
    }

    /// Convert this error into a [`CompileError`], if it is one.
    pub fn compile(self) -> Result<CompileError, TestError> {
        match self.kind {
            TestErrorKind::Compile(err) => Ok(err),
            _ => Err(self),
        }
    }

    /// Convert this error into a [`ShellError`], if it is one.
    pub fn shell(self) -> Result<ShellError, TestError> {
        match self.kind {
            TestErrorKind::Shell(err) => Ok(err),
            _ => Err(self),
        }
    }

    /// Update it's inner location with the call site of this function.
    #[track_caller]
    pub fn update_location(self) -> Self {
        Self {
            location: TestLocation(Location::caller()),
            ..self
        }
    }
}

/// Convenience result type for test helpers.
pub type Result<T = (), E = TestError> = std::result::Result<T, E>;

/// Extensions for asserting error kinds from test helpers.
pub trait TestResultExt: Sized {
    /// Expect the result to be a `Value` equal to the provided input.
    fn expect_value_eq<T: IntoValue>(self, value: T) -> Result;

    /// Expect the result to be an error with a specific [`code`](miette::Diagnostic::code).
    fn expect_error_code_eq(self, code: impl AsRef<str>) -> Result;

    /// Expect the result to be a [`ShellError`].
    fn expect_shell_error(self) -> Result<ShellError>;
    /// Expect the result to be a [`ParseError`].
    fn expect_parse_error(self) -> Result<ParseError>;
    /// Expect the result to be a [`CompileError`].
    fn expect_compile_error(self) -> Result<CompileError>;

    /// Expect the result to be a [`ShellError::Io`].
    fn expect_io_error(self) -> Result<IoError>;
    /// Expect the result to be a [`ShellError::Network`].
    fn expect_network_error(self) -> Result<NetworkError>;
    /// Expect the result to be a [`ShellError::LabeledError`].
    fn expect_labeled_error(self) -> Result<LabeledError>;

    /// Expect the result to be a [`ShellError`].
    #[track_caller]
    fn expect_error(self) -> Result<ShellError> {
        self.expect_shell_error()
    }
}

impl TestResultExt for Result<Value> {
    #[track_caller]
    fn expect_value_eq<T: IntoValue>(self, expected: T) -> Result {
        let expected = expected.into_value(Span::test_data());
        match self {
            Err(err) => Err(err.update_location()),
            Ok(actual) if actual == expected => Ok(()),
            Ok(actual) => Err(TestError {
                location: TestLocation(Location::caller()),
                kind: TestErrorKind::UnexpectedValue {
                    expected,
                    got: actual,
                },
            }),
        }
    }

    #[track_caller]
    fn expect_error_code_eq(self, code: impl AsRef<str>) -> Result {
        let expected = code.as_ref();
        let got = match self {
            Ok(got) => {
                return Err(TestError {
                    location: TestLocation(Location::caller()),
                    kind: TestErrorKind::GotValue { got },
                });
            }
            Err(TestError {
                kind: TestErrorKind::Shell(ref err),
                ..
            }) => err.code(),
            Err(TestError {
                kind: TestErrorKind::Compile(ref err),
                ..
            }) => err.code(),
            Err(TestError {
                kind: TestErrorKind::Parse(ref err),
                ..
            }) => err.code(),
            Err(err) => return Err(err.update_location()),
        };

        let Some(got) = got else {
            return Err(TestError {
                location: TestLocation(Location::caller()),
                kind: TestErrorKind::NoCode {
                    expected: expected.to_string(),
                },
            });
        };

        let got = got.to_string();
        match got == expected {
            true => Ok(()),
            false => Err(TestError {
                location: TestLocation(Location::caller()),
                kind: TestErrorKind::UnexpectedCode {
                    expected: expected.to_string(),
                    got,
                },
            }),
        }
    }

    #[track_caller]
    fn expect_shell_error(self) -> Result<ShellError> {
        match self {
            Ok(got) => Err(TestError {
                location: TestLocation(Location::caller()),
                kind: TestErrorKind::GotValue { got },
            }),
            Err(TestError {
                kind: TestErrorKind::Shell(err),
                ..
            }) => Ok(err),
            Err(err) => Err(err.update_location()),
        }
    }

    #[track_caller]
    fn expect_parse_error(self) -> Result<ParseError> {
        match self {
            Ok(got) => Err(TestError {
                location: TestLocation(Location::caller()),
                kind: TestErrorKind::GotValue { got },
            }),
            Err(TestError {
                kind: TestErrorKind::Parse(err),
                ..
            }) => Ok(err),
            Err(err) => Err(err.update_location()),
        }
    }

    #[track_caller]
    fn expect_compile_error(self) -> Result<CompileError> {
        match self {
            Ok(got) => Err(TestError {
                location: TestLocation(Location::caller()),
                kind: TestErrorKind::GotValue { got },
            }),
            Err(TestError {
                kind: TestErrorKind::Compile(err),
                ..
            }) => Ok(err),
            Err(err) => Err(err.update_location()),
        }
    }

    #[track_caller]
    fn expect_io_error(self) -> Result<IoError> {
        match self {
            Ok(got) => Err(TestError {
                location: TestLocation(Location::caller()),
                kind: TestErrorKind::GotValue { got },
            }),
            Err(TestError {
                kind: TestErrorKind::Shell(ShellError::Io(err)),
                ..
            }) => Ok(err),
            Err(err) => Err(err.update_location()),
        }
    }

    #[track_caller]
    fn expect_network_error(self) -> Result<NetworkError> {
        match self {
            Ok(got) => Err(TestError {
                location: TestLocation(Location::caller()),
                kind: TestErrorKind::GotValue { got },
            }),
            Err(TestError {
                kind: TestErrorKind::Shell(ShellError::Network(err)),
                ..
            }) => Ok(err),
            Err(err) => Err(err.update_location()),
        }
    }

    #[track_caller]
    fn expect_labeled_error(self) -> Result<LabeledError> {
        match self {
            Ok(got) => Err(TestError {
                location: TestLocation(Location::caller()),
                kind: TestErrorKind::GotValue { got },
            }),
            Err(TestError {
                kind: TestErrorKind::Shell(ShellError::LabeledError(err)),
                ..
            }) => Ok(*err),
            Err(err) => Err(err.update_location()),
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
    /// - if `inner` of [`ShellError::Generic`] is empty
    /// - if `sources` of [`ShellError::ChainedError`] is empty
    /// - the error is none of the above types
    ///
    /// So make sure that a [`None`] value is not surprise.
    fn into_inner(self) -> Result<ShellError>;

    /// Extract the [`LabeledError`] from [`ShellError::LabeledError`], if it is one.
    fn into_labeled(self) -> Result<LabeledError>;

    /// Extract the error field from [`ShellError::Generic`], if it is one.
    fn generic_error(self) -> Result<String>;

    /// Extract the message field from [`ShellError::Generic`], if it is one.
    fn generic_msg(self) -> Result<String>;
}

impl ShellErrorExt for ShellError {
    #[track_caller]
    fn into_inner(self) -> Result<ShellError> {
        let no_inner = TestError {
            location: TestLocation(Location::caller()),
            kind: TestErrorKind::NoInner,
        };
        match self {
            ShellError::Generic(err) => err.inner.into_iter().next().ok_or(no_inner),
            ShellError::ChainedError(err) => err.sources_iter().next().ok_or(no_inner),
            _ => Err(no_inner),
        }
    }

    #[track_caller]
    fn into_labeled(self) -> Result<LabeledError> {
        match self {
            ShellError::LabeledError(err) => Ok(*err),
            got => Err(TestError {
                location: TestLocation(Location::caller()),
                kind: TestErrorKind::UnexpectedErrorKind {
                    expected: "Labeled",
                    got,
                },
            }),
        }
    }

    #[track_caller]
    fn generic_error(self) -> Result<String> {
        match self {
            ShellError::Generic(err) => Ok(err.error.into_owned()),
            got => Err(TestError {
                location: TestLocation(Location::caller()),
                kind: TestErrorKind::UnexpectedErrorKind {
                    expected: "Generic",
                    got,
                },
            }),
        }
    }

    #[track_caller]
    fn generic_msg(self) -> Result<String> {
        match self {
            ShellError::Generic(err) => Ok(err.msg.into_owned()),
            got => Err(TestError {
                location: TestLocation(Location::caller()),
                kind: TestErrorKind::UnexpectedErrorKind {
                    expected: "Generic",
                    got,
                },
            }),
        }
    }
}
