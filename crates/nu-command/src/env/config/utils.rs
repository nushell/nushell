use std::collections::HashMap;
use std::path::PathBuf;

use nu_protocol::{
    engine::{EngineState, Stack},
    ShellError, Span, Spanned, Value,
};

use crate::ExternalCommand;

const HELP_MSG: &str = "Nushell's config file can be found with the command: $nu.config-path. \
For more help: (https://nushell.sh/book/configuration.html#configurations-with-built-in-commands)";

fn get_editor_commandline(value: &Value) -> Result<(String, Vec<String>), ShellError> {
    match value {
        Value::String { val, .. } if !val.is_empty() => Ok((val.to_string(), Vec::new())),
        Value::List { vals, .. } if !vals.is_empty() => {
            let mut editor_cmd = vals.iter().map(|l| l.as_string());
            match editor_cmd.next().transpose()? {
                Some(editor) if !editor.is_empty() => {
                    let params = editor_cmd.collect::<Result<_, ShellError>>()?;
                    Ok((editor, params))
                }
                _ => Err(ShellError::GenericError(
                    "Editor's executable is missing".into(),
                    "Set the first element to an executable".into(),
                    Some(value.span()),
                    Some(HELP_MSG.into()),
                    vec![],
                )),
            }
        }
        Value::String { .. } | Value::List { .. } => Err(ShellError::GenericError(
            "Editor can not be empty".into(),
            "Specify one in $EDITOR or $VISUAL".into(),
            Some(value.span()),
            Some(HELP_MSG.into()),
            vec![],
        )),
        x => Err(ShellError::CantConvert {
            to_type: "string or list<string>".into(),
            from_type: x.get_type().to_string(),
            span: value.span(),
            help: None,
        }),
    }
}

pub(crate) fn get_editor(
    engine_state: &EngineState,
    stack: &mut Stack,
    span: Span,
) -> Result<(String, Vec<String>), ShellError> {
    let config = engine_state.get_config();
    let env_vars = stack.get_env_vars(engine_state);
    if !config.buffer_editor.is_empty() {
        Ok((config.buffer_editor.clone(), Vec::new()))
    } else if let Some(value) = env_vars.get("EDITOR").or_else(|| env_vars.get("VISUAL")) {
        get_editor_commandline(value)
    } else {
        Err(ShellError::GenericError(
            "No editor configured".into(),
            "Please specify one via environment variables $EDITOR or $VISUAL".into(),
            Some(span),
            Some(HELP_MSG.into()),
            vec![],
        ))
    }
}

pub(crate) fn gen_command(
    span: Span,
    config_path: PathBuf,
    item: String,
    config_args: Vec<String>,
    env_vars_str: HashMap<String, String>,
) -> ExternalCommand {
    let name = Spanned { item, span };

    let mut args = vec![Spanned {
        item: config_path.to_string_lossy().to_string(),
        span: Span::unknown(),
    }];

    let number_of_args = config_args.len() + 1;

    for arg in config_args {
        args.push(Spanned {
            item: arg,
            span: Span::unknown(),
        })
    }

    ExternalCommand {
        name,
        args,
        arg_keep_raw: vec![false; number_of_args],
        redirect_stdout: false,
        redirect_stderr: false,
        redirect_combine: false,
        env_vars: env_vars_str,
        trim_end_newline: false,
    }
}
