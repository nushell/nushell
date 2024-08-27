//! Implementation of const-evaluation
//!
//! This enables you to assign `const`-constants and execute parse-time code dependent on this.
//! e.g. `source $my_const`
use crate::{
    ast::{Assignment, Block, Call, Expr, Expression, ExternalArgument},
    debugger::{DebugContext, WithoutDebug},
    engine::{EngineState, StateWorkingSet},
    eval_base::Eval,
    Config, HistoryFileFormat, IntoValue, PipelineData, ShellError, Span, Value, VarId,
};
use nu_system::os_info::{get_kernel_version, get_os_arch, get_os_family, get_os_name};
use std::{path::PathBuf, sync::Arc};

// allow IntoValue to refer to this crate als "nu_protocol"
use crate as nu_protocol;

#[derive(Debug, IntoValue)]
#[nu_value(rename_all = "kebab-case")]
pub(crate) struct NuConstant {
    default_config_dir: Result<PathBuf, ShellError>,
    config_path: Result<PathBuf, ShellError>,
    env_path: Result<PathBuf, ShellError>,
    history_path: Result<PathBuf, ShellError>,
    loginshell_path: Result<PathBuf, ShellError>,
    #[cfg(feature = "plugin")]
    plugin_path: Result<PathBuf, ShellError>,
    home_path: Result<PathBuf, ShellError>,
    data_dir: Result<PathBuf, ShellError>,
    cache_dir: Result<PathBuf, ShellError>,
    vendor_autoload_dirs: Vec<PathBuf>,
    temp_path: PathBuf,
    pid: u32,
    os_info: NuConstantOsInfo,
    startup_time: Value, // std::time::Duration cannot be fully represented via Value::Duration
    is_interactive: bool,
    is_login: bool,
    history_enabled: bool,
    current_exe: Result<PathBuf, ShellError>,
}

#[derive(Debug, IntoValue)]
pub(crate) struct NuConstantOsInfo {
    name: &'static str,
    arch: &'static str,
    family: &'static str,
    kernel_version: String,
}

impl NuConstant {
    /// Create a Value for `$nu`.
    pub(crate) fn create(engine_state: &EngineState, span: Span) -> Value {
        fn canonicalize_path(
            engine_state: &EngineState,
            path: impl Into<PathBuf>,
            join: impl Into<Option<&'static str>>,
        ) -> PathBuf {
            let mut path = path.into();
            let join = join.into();

            #[allow(deprecated)]
            let cwd = engine_state.current_work_dir();

            if let Some(join) = join {
                path.push(join);
            }

            if path.exists() {
                match nu_path::canonicalize_with(&path, cwd) {
                    Ok(canon_path) => canon_path,
                    Err(_) => path,
                }
            } else {
                path
            }
        }

        // shortcut for function calls
        let es = engine_state;

        let config_dir = match nu_path::config_dir() {
            Some(path) => Ok(canonicalize_path(es, path.into_std_path_buf(), "nushell")),
            None => Err(ShellError::ConfigDirNotFound { span: Some(span) }),
        };

        let default_config_dir = config_dir.clone();

        let config_path = match (
            engine_state.get_config_path("config-path"),
            config_dir.clone(),
        ) {
            (Some(path), _) => Ok(canonicalize_path(es, path, None)),
            (None, Err(e)) => Err(e),
            (None, Ok(path)) => Ok(canonicalize_path(es, path, "config.nu")),
        };

        let env_path = match (engine_state.get_config_path("env-path"), config_dir.clone()) {
            (Some(path), _) => Ok(canonicalize_path(es, path, None)),
            (None, Err(e)) => Err(e),
            (None, Ok(path)) => Ok(canonicalize_path(es, path, "config.nu")),
        };

        let history_path = config_dir.clone().map(|path| {
            let file = match engine_state.config.history.file_format {
                HistoryFileFormat::Sqlite => "history.sqlite3",
                HistoryFileFormat::PlainText => "history.txt",
            };
            canonicalize_path(es, path, file)
        });

        let loginshell_path = config_dir
            .clone()
            .map(|path| canonicalize_path(es, path, "login.nu"));

        #[cfg(feature = "plugin")]
        let plugin_path = match (&engine_state.plugin_path, config_dir) {
            (Some(path), _) => Ok(canonicalize_path(es, path, None)),
            (None, Err(e)) => Err(e.to_owned()),
            (None, Ok(path)) => Ok(canonicalize_path(es, path, "plugin.msgpackz")),
        };

        let home_path = match nu_path::home_dir() {
            Some(path) => Ok(canonicalize_path(es, path, None)),
            None => Err(ShellError::IOError {
                msg: "Could not get home path".into(),
            }),
        };

        let data_dir = match nu_path::data_dir() {
            Some(path) => Ok(canonicalize_path(es, path, "nushell")),
            None => Err(ShellError::IOError {
                msg: "Could not get data path".into(),
            }),
        };

        let cache_dir = match nu_path::cache_dir() {
            Some(path) => Ok(canonicalize_path(es, path, "nushell")),
            None => Err(ShellError::IOError {
                msg: "Could not get cache path".into(),
            }),
        };

        let vendor_autoload_dirs = get_vendor_autoload_dirs(engine_state);
        let temp_path = canonicalize_path(es, std::env::temp_dir(), None);
        let pid = std::process::id();

        let os_info = NuConstantOsInfo {
            name: get_os_name(),
            arch: get_os_arch(),
            family: get_os_family(),
            kernel_version: get_kernel_version(),
        };

        let startup_time = Value::duration(engine_state.get_startup_time(), span);
        let is_interactive = engine_state.is_interactive;
        let is_login = engine_state.is_login;
        let history_enabled = engine_state.history_enabled;

        let current_exe = std::env::current_exe().map_err(|_| ShellError::IOError {
            msg: "Could not get current executable path".to_string(),
        });

        NuConstant {
            default_config_dir,
            config_path,
            env_path,
            history_path,
            loginshell_path,
            #[cfg(feature = "plugin")]
            plugin_path,
            home_path,
            data_dir,
            cache_dir,
            vendor_autoload_dirs,
            temp_path,
            pid,
            os_info,
            startup_time,
            is_interactive,
            is_login,
            history_enabled,
            current_exe,
        }
        .into_value(span)
    }
}

pub fn get_vendor_autoload_dirs(_engine_state: &EngineState) -> Vec<PathBuf> {
    // load order for autoload dirs
    // /Library/Application Support/nushell/vendor/autoload on macOS
    // <dir>/nushell/vendor/autoload for every dir in XDG_DATA_DIRS in reverse order on platforms other than windows. If XDG_DATA_DIRS is not set, it falls back to <PREFIX>/share if PREFIX ends in local, or <PREFIX>/local/share:<PREFIX>/share otherwise. If PREFIX is not set, fall back to /usr/local/share:/usr/share.
    // %ProgramData%\nushell\vendor\autoload on windows
    // NU_VENDOR_AUTOLOAD_DIR from compile time, if env var is set at compile time
    // if on macOS, additionally check XDG_DATA_HOME, which `dirs` is only doing on Linux
    // <data_dir>/nushell/vendor/autoload of the current user according to the `dirs` crate
    // NU_VENDOR_AUTOLOAD_DIR at runtime, if env var is set

    let into_autoload_path_fn = |mut path: PathBuf| {
        path.push("nushell");
        path.push("vendor");
        path.push("autoload");
        path
    };

    let mut dirs = Vec::new();

    let mut append_fn = |path: PathBuf| {
        if !dirs.contains(&path) {
            dirs.push(path)
        }
    };

    #[cfg(target_os = "macos")]
    std::iter::once("/Library/Application Support")
        .map(PathBuf::from)
        .map(into_autoload_path_fn)
        .for_each(&mut append_fn);
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;

        std::env::var_os("XDG_DATA_DIRS")
            .or_else(|| {
                option_env!("PREFIX").map(|prefix| {
                    if prefix.ends_with("local") {
                        std::ffi::OsString::from(format!("{prefix}/share"))
                    } else {
                        std::ffi::OsString::from(format!("{prefix}/local/share:{prefix}/share"))
                    }
                })
            })
            .unwrap_or_else(|| std::ffi::OsString::from("/usr/local/share/:/usr/share/"))
            .as_encoded_bytes()
            .split(|b| *b == b':')
            .map(|split| into_autoload_path_fn(PathBuf::from(std::ffi::OsStr::from_bytes(split))))
            .rev()
            .for_each(&mut append_fn);
    }

    #[cfg(target_os = "windows")]
    dirs_sys::known_folder(windows_sys::Win32::UI::Shell::FOLDERID_ProgramData)
        .into_iter()
        .map(into_autoload_path_fn)
        .for_each(&mut append_fn);

    option_env!("NU_VENDOR_AUTOLOAD_DIR")
        .into_iter()
        .map(PathBuf::from)
        .for_each(&mut append_fn);

    #[cfg(target_os = "macos")]
    std::env::var("XDG_DATA_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            dirs::home_dir().map(|mut home| {
                home.push(".local");
                home.push("share");
                home
            })
        })
        .map(into_autoload_path_fn)
        .into_iter()
        .for_each(&mut append_fn);

    dirs::data_dir()
        .into_iter()
        .map(into_autoload_path_fn)
        .for_each(&mut append_fn);

    std::env::var_os("NU_VENDOR_AUTOLOAD_DIR")
        .into_iter()
        .map(PathBuf::from)
        .for_each(&mut append_fn);

    dirs
}

fn eval_const_call(
    working_set: &StateWorkingSet,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let decl = working_set.get_decl(call.decl_id);

    if !decl.is_const() {
        return Err(ShellError::NotAConstCommand { span: call.head });
    }

    if !decl.is_known_external() && call.named_iter().any(|(flag, _, _)| flag.item == "help") {
        // It would require re-implementing get_full_help() for const evaluation. Assuming that
        // getting help messages at parse-time is rare enough, we can simply disallow it.
        return Err(ShellError::NotAConstHelp { span: call.head });
    }

    decl.run_const(working_set, &call.into(), input)
}

pub fn eval_const_subexpression(
    working_set: &StateWorkingSet,
    block: &Block,
    mut input: PipelineData,
    span: Span,
) -> Result<PipelineData, ShellError> {
    for pipeline in block.pipelines.iter() {
        for element in pipeline.elements.iter() {
            if element.redirection.is_some() {
                return Err(ShellError::NotAConstant { span });
            }

            input = eval_constant_with_input(working_set, &element.expr, input)?
        }
    }

    Ok(input)
}

pub fn eval_constant_with_input(
    working_set: &StateWorkingSet,
    expr: &Expression,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    match &expr.expr {
        Expr::Call(call) => eval_const_call(working_set, call, input),
        Expr::Subexpression(block_id) => {
            let block = working_set.get_block(*block_id);
            eval_const_subexpression(working_set, block, input, expr.span(&working_set))
        }
        _ => eval_constant(working_set, expr).map(|v| PipelineData::Value(v, None)),
    }
}

/// Evaluate a constant value at parse time
pub fn eval_constant(
    working_set: &StateWorkingSet,
    expr: &Expression,
) -> Result<Value, ShellError> {
    // TODO: Allow debugging const eval
    <EvalConst as Eval>::eval::<WithoutDebug>(working_set, &mut (), expr)
}

struct EvalConst;

impl Eval for EvalConst {
    type State<'a> = &'a StateWorkingSet<'a>;

    type MutState = ();

    fn get_config(state: Self::State<'_>, _: &mut ()) -> Arc<Config> {
        state.get_config().clone()
    }

    fn eval_filepath(
        _: &StateWorkingSet,
        _: &mut (),
        path: String,
        _: bool,
        span: Span,
    ) -> Result<Value, ShellError> {
        Ok(Value::string(path, span))
    }

    fn eval_directory(
        _: &StateWorkingSet,
        _: &mut (),
        _: String,
        _: bool,
        span: Span,
    ) -> Result<Value, ShellError> {
        Err(ShellError::NotAConstant { span })
    }

    fn eval_var(
        working_set: &StateWorkingSet,
        _: &mut (),
        var_id: VarId,
        span: Span,
    ) -> Result<Value, ShellError> {
        match working_set.get_variable(var_id).const_val.as_ref() {
            Some(val) => Ok(val.clone()),
            None => Err(ShellError::NotAConstant { span }),
        }
    }

    fn eval_call<D: DebugContext>(
        working_set: &StateWorkingSet,
        _: &mut (),
        call: &Call,
        span: Span,
    ) -> Result<Value, ShellError> {
        // TODO: Allow debugging const eval
        // TODO: eval.rs uses call.head for the span rather than expr.span
        eval_const_call(working_set, call, PipelineData::empty())?.into_value(span)
    }

    fn eval_external_call(
        _: &StateWorkingSet,
        _: &mut (),
        _: &Expression,
        _: &[ExternalArgument],
        span: Span,
    ) -> Result<Value, ShellError> {
        // TODO: It may be more helpful to give not_a_const_command error
        Err(ShellError::NotAConstant { span })
    }

    fn eval_collect<D: DebugContext>(
        _: &StateWorkingSet,
        _: &mut (),
        _var_id: VarId,
        expr: &Expression,
    ) -> Result<Value, ShellError> {
        Err(ShellError::NotAConstant { span: expr.span })
    }

    fn eval_subexpression<D: DebugContext>(
        working_set: &StateWorkingSet,
        _: &mut (),
        block_id: usize,
        span: Span,
    ) -> Result<Value, ShellError> {
        // TODO: Allow debugging const eval
        let block = working_set.get_block(block_id);
        eval_const_subexpression(working_set, block, PipelineData::empty(), span)?.into_value(span)
    }

    fn regex_match(
        _: &StateWorkingSet,
        _op_span: Span,
        _: &Value,
        _: &Value,
        _: bool,
        expr_span: Span,
    ) -> Result<Value, ShellError> {
        Err(ShellError::NotAConstant { span: expr_span })
    }

    fn eval_assignment<D: DebugContext>(
        _: &StateWorkingSet,
        _: &mut (),
        _: &Expression,
        _: &Expression,
        _: Assignment,
        _op_span: Span,
        expr_span: Span,
    ) -> Result<Value, ShellError> {
        // TODO: Allow debugging const eval
        Err(ShellError::NotAConstant { span: expr_span })
    }

    fn eval_row_condition_or_closure(
        _: &StateWorkingSet,
        _: &mut (),
        _: usize,
        span: Span,
    ) -> Result<Value, ShellError> {
        Err(ShellError::NotAConstant { span })
    }

    fn eval_overlay(_: &StateWorkingSet, span: Span) -> Result<Value, ShellError> {
        Err(ShellError::NotAConstant { span })
    }

    fn unreachable(working_set: &StateWorkingSet, expr: &Expression) -> Result<Value, ShellError> {
        Err(ShellError::NotAConstant {
            span: expr.span(&working_set),
        })
    }
}
