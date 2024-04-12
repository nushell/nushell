use nu_protocol::{
    engine::{EngineState, Stack, StateWorkingSet},
    report_error, Range, ShellError, Span, Value,
};
use std::{ops::Bound, path::PathBuf};

pub fn get_init_cwd() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| {
        std::env::var("PWD")
            .map(Into::into)
            .unwrap_or_else(|_| nu_path::home_dir().unwrap_or_default())
    })
}

pub fn get_guaranteed_cwd(engine_state: &EngineState, stack: &Stack) -> PathBuf {
    nu_engine::env::current_dir(engine_state, stack).unwrap_or_else(|e| {
        let working_set = StateWorkingSet::new(engine_state);
        report_error(&working_set, &e);
        crate::util::get_init_cwd()
    })
}

type MakeRangeError = fn(&str, Span) -> ShellError;

pub fn process_range(range: &Range) -> Result<(isize, isize), MakeRangeError> {
    match range {
        Range::IntRange(range) => {
            let start = range.start().try_into().unwrap_or(0);
            let end = match range.end() {
                Bound::Included(v) => v as isize,
                Bound::Excluded(v) => (v - 1) as isize,
                Bound::Unbounded => isize::MAX,
            };
            Ok((start, end))
        }
        Range::FloatRange(_) => Err(|msg, span| ShellError::TypeMismatch {
            err_message: msg.to_string(),
            span,
        }),
    }
}

const HELP_MSG: &str = "Nushell's config file can be found with the command: $nu.config-path. \
For more help: (https://nushell.sh/book/configuration.html#configurations-with-built-in-commands)";

fn get_editor_commandline(
    value: &Value,
    var_name: &str,
) -> Result<(String, Vec<String>), ShellError> {
    match value {
        Value::String { val, .. } if !val.is_empty() => Ok((val.to_string(), Vec::new())),
        Value::List { vals, .. } if !vals.is_empty() => {
            let mut editor_cmd = vals.iter().map(|l| l.coerce_string());
            match editor_cmd.next().transpose()? {
                Some(editor) if !editor.is_empty() => {
                    let params = editor_cmd.collect::<Result<_, ShellError>>()?;
                    Ok((editor, params))
                }
                _ => Err(ShellError::GenericError {
                    error: "Editor executable is missing".into(),
                    msg: "Set the first element to an executable".into(),
                    span: Some(value.span()),
                    help: Some(HELP_MSG.into()),
                    inner: vec![],
                }),
            }
        }
        Value::String { .. } | Value::List { .. } => Err(ShellError::GenericError {
            error: format!("{var_name} should be a non-empty string or list<String>"),
            msg: "Specify an executable here".into(),
            span: Some(value.span()),
            help: Some(HELP_MSG.into()),
            inner: vec![],
        }),
        x => Err(ShellError::CantConvert {
            to_type: "string or list<string>".into(),
            from_type: x.get_type().to_string(),
            span: value.span(),
            help: None,
        }),
    }
}

pub fn get_editor(
    engine_state: &EngineState,
    stack: &Stack,
    span: Span,
) -> Result<(String, Vec<String>), ShellError> {
    let config = engine_state.get_config();
    let env_vars = stack.get_env_vars(engine_state);

    if let Ok(buff_editor) =
        get_editor_commandline(&config.buffer_editor, "$env.config.buffer_editor")
    {
        Ok(buff_editor)
    } else if let Some(value) = env_vars.get("EDITOR") {
        get_editor_commandline(value, "$env.EDITOR")
    } else if let Some(value) = env_vars.get("VISUAL") {
        get_editor_commandline(value, "$env.VISUAL")
    } else {
        Err(ShellError::GenericError {
            error: "No editor configured".into(),
            msg:
                "Please specify one via `$env.config.buffer_editor` or `$env.EDITOR`/`$env.VISUAL`"
                    .into(),
            span: Some(span),
            help: Some(HELP_MSG.into()),
            inner: vec![],
        })
    }
}
