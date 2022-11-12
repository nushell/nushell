use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use nu_engine::CallExt;
use nu_protocol::{
    engine::Command, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape,
    Type, Value,
};

use super::PathSubcommandArguments;

struct Arguments {
    columns: Option<Vec<String>>,
    append: Vec<Spanned<String>>,
}

impl PathSubcommandArguments for Arguments {
    fn get_columns(&self) -> Option<Vec<String>> {
        self.columns.clone()
    }
}

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
                (Type::Table(vec![]), Type::List(Box::new(Type::String))),
            ])
            .named(
                "columns",
                SyntaxShape::Table,
                "For a record or table input, join strings at the given columns",
                Some('c'),
            )
            .rest("append", SyntaxShape::String, "Path to append to the input")
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
        engine_state: &nu_protocol::engine::EngineState,
        stack: &mut nu_protocol::engine::Stack,
        call: &nu_protocol::ast::Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let args = Arguments {
            columns: call.get_flag(engine_state, stack, "columns")?,
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
            _ => Err(ShellError::UnsupportedInput(
                "Input data is not supported by this command.".to_string(),
                head,
            )),
        }
    }

    #[cfg(windows)]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Append a filename to a path",
                example: r"'C:\Users\viking' | path join spam.txt",
                result: Some(Value::test_string(r"C:\Users\viking\spam.txt")),
            },
            Example {
                description: "Append a filename to a path",
                example: r"'C:\Users\viking' | path join spams this_spam.txt",
                result: Some(Value::test_string(r"C:\Users\viking\spams\this_spam.txt")),
            },
            Example {
                description: "Append a filename to a path inside a column",
                example: r"ls | path join spam.txt -c [ name ]",
                result: None,
            },
            Example {
                description: "Join a list of parts into a path",
                example: r"[ 'C:' '\' 'Users' 'viking' 'spam.txt' ] | path join",
                result: Some(Value::test_string(r"C:\Users\viking\spam.txt")),
            },
            Example {
                description: "Join a structured path into a path",
                example: r"[ [parent stem extension]; ['C:\Users\viking' 'spam' 'txt']] | path join",
                result: Some(Value::List {
                    vals: vec![Value::test_string(r"C:\Users\viking\spam.txt")],
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
                result: Some(Value::test_string(r"/home/viking/spam.txt")),
            },
            Example {
                description: "Append a filename to a path",
                example: r"'/home/viking' | path join spams this_spam.txt",
                result: Some(Value::test_string(r"/home/viking/spams/this_spam.txt")),
            },
            Example {
                description: "Append a filename to a path inside a column",
                example: r"ls | path join spam.txt -c [ name ]",
                result: None,
            },
            Example {
                description: "Join a list of parts into a path",
                example: r"[ '/' 'home' 'viking' 'spam.txt' ] | path join",
                result: Some(Value::test_string(r"/home/viking/spam.txt")),
            },
            Example {
                description: "Join a structured path into a path",
                example: r"[[ parent stem extension ]; [ '/home/viking' 'spam' 'txt' ]] | path join",
                result: Some(Value::List {
                    vals: vec![Value::test_string(r"/home/viking/spam.txt")],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn handle_value(v: Value, args: &Arguments, head: Span) -> Value {
    match v {
        Value::String { ref val, span } => join_single(Path::new(val), span, args),
        Value::Record { cols, vals, span } => join_record(&cols, &vals, span, args),
        Value::List { vals, span } => join_list(&vals, span, args),

        _ => super::handle_invalid_values(v, head),
    }
}

fn join_single(path: &Path, span: Span, args: &Arguments) -> Value {
    let mut result = path.to_path_buf();
    for path_to_append in &args.append {
        result.push(&path_to_append.item)
    }

    Value::string(result.to_string_lossy(), span)
}

fn join_list(parts: &[Value], span: Span, args: &Arguments) -> Value {
    let path: Result<PathBuf, ShellError> = parts.iter().map(Value::as_string).collect();

    match path {
        Ok(ref path) => join_single(path, span, args),
        Err(_) => {
            let records: Result<Vec<_>, ShellError> = parts.iter().map(Value::as_record).collect();
            match records {
                Ok(vals) => {
                    let vals = vals
                        .iter()
                        .map(|(k, v)| join_record(k, v, span, args))
                        .collect();

                    Value::List { vals, span }
                }
                Err(_) => Value::Error {
                    error: ShellError::PipelineMismatch("string or record".into(), span, span),
                },
            }
        }
    }
}

fn join_record(cols: &[String], vals: &[Value], span: Span, args: &Arguments) -> Value {
    if args.columns.is_some() {
        super::operate(
            &join_single,
            args,
            Value::Record {
                cols: cols.to_vec(),
                vals: vals.to_vec(),
                span,
            },
            span,
        )
    } else {
        match merge_record(cols, vals, span) {
            Ok(p) => join_single(p.as_path(), span, args),
            Err(error) => Value::Error { error },
        }
    }
}

fn merge_record(cols: &[String], vals: &[Value], span: Span) -> Result<PathBuf, ShellError> {
    for key in cols {
        if !super::ALLOWED_COLUMNS.contains(&key.as_str()) {
            let allowed_cols = super::ALLOWED_COLUMNS.join(", ");
            let msg = format!(
                "Column '{}' is not valid for a structured path. Allowed columns are: {}",
                key, allowed_cols
            );
            return Err(ShellError::UnsupportedInput(msg, span));
        }
    }

    let entries: HashMap<&str, &Value> = cols.iter().map(String::as_str).zip(vals).collect();
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
