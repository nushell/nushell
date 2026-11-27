use nu_protocol::{
    Range, ShellError, Span, Value,
    engine::{EngineState, Stack},
    shell_error::generic::GenericError,
};
use std::ops::Bound;

type MakeRangeError = fn(&str, Span) -> ShellError;

/// Returns a inclusive pair of boundary in given `range`.
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
        Value::String { val, .. } if !val.trim().is_empty() => {
            let (tokens, _) = nu_parser::lex(val.as_bytes(), 0, &[], &[], true);
            let mut words = tokens.into_iter().map(|token| {
                let bytes = &val.as_bytes()[token.span.start..token.span.end];
                nu_parser::unescape_unquote_string(bytes, token.span).0
            });
            match words.next() {
                Some(editor) if !editor.is_empty() => {
                    let params = words.collect();
                    Ok((editor, params))
                }
                _ => Err(ShellError::Generic(
                    GenericError::new(
                        "Editor executable is missing",
                        "Set the editor to an executable",
                        value.span(),
                    )
                    .with_help(HELP_MSG),
                )),
            }
        }
        Value::List { vals, .. } if !vals.is_empty() => {
            let mut editor_cmd = vals.iter().map(|l| l.coerce_string());
            match editor_cmd.next().transpose()? {
                Some(editor) if !editor.is_empty() => {
                    let params = editor_cmd.collect::<Result<_, ShellError>>()?;
                    Ok((editor, params))
                }
                _ => Err(ShellError::Generic(
                    GenericError::new(
                        "Editor executable is missing",
                        "Set the first element to an executable",
                        value.span(),
                    )
                    .with_help(HELP_MSG),
                )),
            }
        }
        Value::String { .. } | Value::List { .. } => Err(ShellError::Generic(
            GenericError::new(
                format!("{var_name} should be a non-empty string or list<String>"),
                "Specify an executable here",
                value.span(),
            )
            .with_help(HELP_MSG),
        )),
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
    let config = stack.get_config(engine_state);

    if let Ok(buff_editor) =
        get_editor_commandline(&config.buffer_editor, "$env.config.buffer_editor")
    {
        Ok(buff_editor)
    } else if let Some(value) = stack.get_env_var(engine_state, "VISUAL") {
        get_editor_commandline(value, "$env.VISUAL")
    } else if let Some(value) = stack.get_env_var(engine_state, "EDITOR") {
        get_editor_commandline(value, "$env.EDITOR")
    } else {
        Err(ShellError::Generic(
            GenericError::new(
                "No editor configured",
                "Please specify one via `$env.config.buffer_editor` or `$env.EDITOR`/`$env.VISUAL`",
                span,
            )
            .with_help(HELP_MSG),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_commandline_single_string() {
        let val = Value::test_string("vi");
        let (cmd, args) = get_editor_commandline(&val, "$env.EDITOR").unwrap();
        assert_eq!(cmd, "vi");
        assert!(args.is_empty());
    }

    #[test]
    fn test_editor_commandline_string_with_args() {
        let val = Value::test_string("mg -n");
        let (cmd, args) = get_editor_commandline(&val, "$env.config.buffer_editor").unwrap();
        assert_eq!(cmd, "mg");
        assert_eq!(args, vec!["-n"]);
    }

    #[test]
    fn test_editor_commandline_string_multiple_args() {
        let val = Value::test_string("emacsclient -s light -t");
        let (cmd, args) = get_editor_commandline(&val, "$env.EDITOR").unwrap();
        assert_eq!(cmd, "emacsclient");
        assert_eq!(args, vec!["-s", "light", "-t"]);
    }

    #[test]
    fn test_editor_commandline_quoted_path_with_spaces() {
        let val = Value::test_string("\"/path to/my editor\" -C");
        let (cmd, args) = get_editor_commandline(&val, "$env.EDITOR").unwrap();
        assert_eq!(cmd, "/path to/my editor");
        assert_eq!(args, vec!["-C"]);
    }

    #[test]
    fn test_editor_commandline_list() {
        let val = Value::test_list(vec![Value::test_string("mg"), Value::test_string("-n")]);
        let (cmd, args) = get_editor_commandline(&val, "$env.config.buffer_editor").unwrap();
        assert_eq!(cmd, "mg");
        assert_eq!(args, vec!["-n"]);
    }

    #[test]
    fn test_editor_commandline_empty_string_fails() {
        let val = Value::test_string("   ");
        assert!(get_editor_commandline(&val, "$env.EDITOR").is_err());
    }
}
