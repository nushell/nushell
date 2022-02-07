use std::path::Path;

use nu_engine::CallExt;
use nu_protocol::{engine::Command, Example, ShellError, Signature, Span, SyntaxShape, Value};

use super::PathSubcommandArguments;

struct Arguments {
    columns: Option<Vec<String>>,
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
        "path split"
    }

    fn signature(&self) -> Signature {
        Signature::build("path split").named(
            "columns",
            SyntaxShape::Table,
            "Optionally operate by column path",
            Some('c'),
        )
    }

    fn usage(&self) -> &str {
        "Split a path into parts by a separator."
    }

    fn run(
        &self,
        engine_state: &nu_protocol::engine::EngineState,
        stack: &mut nu_protocol::engine::Stack,
        call: &nu_protocol::ast::Call,
        input: nu_protocol::PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        let head = call.head;
        let args = Arguments {
            columns: call.get_flag(engine_state, stack, "columns")?,
        };

        input.map(
            move |value| super::operate(&split, &args, value, head),
            engine_state.ctrlc.clone(),
        )
    }

    #[cfg(windows)]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Split a path into parts",
                example: r"'C:\Users\viking\spam.txt' | path split",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_string("C:"),
                        Value::test_string(r"\"),
                        Value::test_string("Users"),
                        Value::test_string("viking"),
                        Value::test_string("spam.txt"),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Split all paths under the 'name' column",
                example: r"ls ('.' | path expand) | path split -c [ name ]",
                result: None,
            },
        ]
    }

    #[cfg(not(windows))]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Split a path into parts",
                example: r"'/home/viking/spam.txt' | path split",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_string("/"),
                        Value::test_string("home"),
                        Value::test_string("viking"),
                        Value::test_string("spam.txt"),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Split all paths under the 'name' column",
                example: r"ls ('.' | path expand) | path split -c [ name ]",
                result: None,
            },
        ]
    }
}

fn split(path: &Path, span: Span, _: &Arguments) -> Value {
    Value::List {
        vals: path
            .components()
            .map(|comp| {
                let s = comp.as_os_str().to_string_lossy();
                Value::string(s, span)
            })
            .collect(),
        span,
    }
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
