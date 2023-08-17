use std::collections::HashMap;
use std::path::{Path, PathBuf};

use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::{
    engine::Command, Category, Example, PipelineData, ShellError, Signature, Span, Spanned,
    SpannedValue, SyntaxShape, Type,
};

use super::PathSubcommandArguments;

struct Arguments {
    append: Vec<Spanned<String>>,
}

impl PathSubcommandArguments for Arguments {}

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "path join"
    }

    fn signature(&self) -> Signature {
        Signature::build("path join")
            .input_output_types(vec![
                (Type::String, Type::String),
                (Type::List(Box::new(Type::String)), Type::String),
                (Type::Record(vec![]), Type::String),
                (Type::Table(vec![]), Type::List(Box::new(Type::String))),
            ])
            .rest("append", SyntaxShape::String, "Path to append to the input")
            .category(Category::Path)
    }

    fn usage(&self) -> &str {
        "Join a structured path or a list of path parts."
    }

    fn extra_usage(&self) -> &str {
        r#"Optionally, append an additional path to the result. It is designed to accept
the output of 'path parse' and 'path split' subcommands."#
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let args = Arguments {
            append: call.rest(engine_state, stack, 0)?,
        };

        let metadata = input.metadata();

        match input {
            PipelineData::Value(val, md) => {
                Ok(PipelineData::Value(handle_value(val, &args, head), md))
            }
            PipelineData::ListStream(..) => Ok(PipelineData::Value(
                handle_value(input.into_value(head), &args, head),
                metadata,
            )),
            PipelineData::Empty { .. } => Err(ShellError::PipelineEmpty { dst_span: head }),
            _ => Err(ShellError::UnsupportedInput(
                "Input value cannot be joined".to_string(),
                "value originates from here".into(),
                head,
                input.span().unwrap_or(call.head),
            )),
        }
    }

    #[cfg(windows)]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Append a filename to a path",
                example: r"'C:\Users\viking' | path join spam.txt",
                result: Some(SpannedValue::test_string(r"C:\Users\viking\spam.txt")),
            },
            Example {
                description: "Append a filename to a path",
                example: r"'C:\Users\viking' | path join spams this_spam.txt",
                result: Some(SpannedValue::test_string(
                    r"C:\Users\viking\spams\this_spam.txt",
                )),
            },
            Example {
                description: "Join a list of parts into a path",
                example: r"[ 'C:' '\' 'Users' 'viking' 'spam.txt' ] | path join",
                result: Some(SpannedValue::test_string(r"C:\Users\viking\spam.txt")),
            },
            Example {
                description: "Join a structured path into a path",
                example: r"{ parent: 'C:\Users\viking', stem: 'spam', extension: 'txt' } | path join",
                result: Some(SpannedValue::test_string(r"C:\Users\viking\spam.txt")),
            },
            Example {
                description: "Join a table of structured paths into a list of paths",
                example: r"[ [parent stem extension]; ['C:\Users\viking' 'spam' 'txt']] | path join",
                result: Some(SpannedValue::List {
                    vals: vec![SpannedValue::test_string(r"C:\Users\viking\spam.txt")],
                    span: Span::test_data(),
                }),
            },
        ]
    }

    #[cfg(not(windows))]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Append a filename to a path",
                example: r"'/home/viking' | path join spam.txt",
                result: Some(SpannedValue::test_string(r"/home/viking/spam.txt")),
            },
            Example {
                description: "Append a filename to a path",
                example: r"'/home/viking' | path join spams this_spam.txt",
                result: Some(SpannedValue::test_string(
                    r"/home/viking/spams/this_spam.txt",
                )),
            },
            Example {
                description: "Join a list of parts into a path",
                example: r"[ '/' 'home' 'viking' 'spam.txt' ] | path join",
                result: Some(SpannedValue::test_string(r"/home/viking/spam.txt")),
            },
            Example {
                description: "Join a structured path into a path",
                example: r"{ parent: '/home/viking', stem: 'spam', extension: 'txt' } | path join",
                result: Some(SpannedValue::test_string(r"/home/viking/spam.txt")),
            },
            Example {
                description: "Join a table of structured paths into a list of paths",
                example: r"[[ parent stem extension ]; [ '/home/viking' 'spam' 'txt' ]] | path join",
                result: Some(SpannedValue::List {
                    vals: vec![SpannedValue::test_string(r"/home/viking/spam.txt")],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn handle_value(v: SpannedValue, args: &Arguments, head: Span) -> SpannedValue {
    match v {
        SpannedValue::String { ref val, .. } => join_single(Path::new(val), head, args),
        SpannedValue::Record { cols, vals, span } => join_record(&cols, &vals, head, span, args),
        SpannedValue::List { vals, span } => join_list(&vals, head, span, args),

        _ => super::handle_invalid_values(v, head),
    }
}

fn join_single(path: &Path, head: Span, args: &Arguments) -> SpannedValue {
    let mut result = path.to_path_buf();
    for path_to_append in &args.append {
        result.push(&path_to_append.item)
    }

    SpannedValue::string(result.to_string_lossy(), head)
}

fn join_list(parts: &[SpannedValue], head: Span, span: Span, args: &Arguments) -> SpannedValue {
    let path: Result<PathBuf, ShellError> = parts.iter().map(SpannedValue::as_string).collect();

    match path {
        Ok(ref path) => join_single(path, head, args),
        Err(_) => {
            let records: Result<Vec<_>, ShellError> =
                parts.iter().map(SpannedValue::as_record).collect();
            match records {
                Ok(vals) => {
                    let vals = vals
                        .iter()
                        .map(|(k, v)| join_record(k, v, head, span, args))
                        .collect();

                    SpannedValue::List { vals, span }
                }
                Err(_) => SpannedValue::Error {
                    error: Box::new(ShellError::PipelineMismatch {
                        exp_input_type: "string or record".into(),
                        dst_span: head,
                        src_span: span,
                    }),
                },
            }
        }
    }
}

fn join_record(
    cols: &[String],
    vals: &[SpannedValue],
    head: Span,
    span: Span,
    args: &Arguments,
) -> SpannedValue {
    match merge_record(cols, vals, head, span) {
        Ok(p) => join_single(p.as_path(), head, args),
        Err(error) => SpannedValue::Error {
            error: Box::new(error),
        },
    }
}

fn merge_record(
    cols: &[String],
    vals: &[SpannedValue],
    head: Span,
    span: Span,
) -> Result<PathBuf, ShellError> {
    for key in cols {
        if !super::ALLOWED_COLUMNS.contains(&key.as_str()) {
            let allowed_cols = super::ALLOWED_COLUMNS.join(", ");
            return Err(ShellError::UnsupportedInput(
                format!(
                    "Column '{key}' is not valid for a structured path. Allowed columns on this platform are: {allowed_cols}"
                ),
                "value originates from here".into(),
                head,
                span
            ));
        }
    }

    let entries: HashMap<&str, &SpannedValue> = cols.iter().map(String::as_str).zip(vals).collect();
    let mut result = PathBuf::new();

    #[cfg(windows)]
    if let Some(val) = entries.get("prefix") {
        let p = val.as_string()?;
        if !p.is_empty() {
            result.push(p);
        }
    }

    if let Some(val) = entries.get("parent") {
        let p = val.as_string()?;
        if !p.is_empty() {
            result.push(p);
        }
    }

    let mut basename = String::new();
    if let Some(val) = entries.get("stem") {
        let p = val.as_string()?;
        if !p.is_empty() {
            basename.push_str(&p);
        }
    }

    if let Some(val) = entries.get("extension") {
        let p = val.as_string()?;
        if !p.is_empty() {
            basename.push('.');
            basename.push_str(&p);
        }
    }

    if !basename.is_empty() {
        result.push(basename);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
