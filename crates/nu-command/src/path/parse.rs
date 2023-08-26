use std::path::Path;

use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{EngineState, Stack, StateWorkingSet};
use nu_protocol::{
    engine::Command, Category, Example, PipelineData, Record, ShellError, Signature, Span, Spanned,
    SyntaxShape, Type, Value,
};

use super::PathSubcommandArguments;

struct Arguments {
    extension: Option<Spanned<String>>,
}

impl PathSubcommandArguments for Arguments {}

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "path parse"
    }

    fn signature(&self) -> Signature {
        Signature::build("path parse")
            .input_output_types(vec![
                (Type::String, Type::Record(vec![])),
                (Type::List(Box::new(Type::String)), Type::Table(vec![])),
            ])
            .named(
                "extension",
                SyntaxShape::String,
                "Manually supply the extension (without the dot)",
                Some('e'),
            )
            .category(Category::Path)
    }

    fn usage(&self) -> &str {
        "Convert a path into structured data."
    }

    fn extra_usage(&self) -> &str {
        r#"Each path is split into a table with 'parent', 'stem' and 'extension' fields.
On Windows, an extra 'prefix' column is added."#
    }

    fn is_const(&self) -> bool {
        true
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
            extension: call.get_flag(engine_state, stack, "extension")?,
        };

        // This doesn't match explicit nulls
        if matches!(input, PipelineData::Empty) {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        input.map(
            move |value| super::operate(&parse, &args, value, head),
            engine_state.ctrlc.clone(),
        )
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let args = Arguments {
            extension: call.get_flag_const(working_set, "extension")?,
        };

        // This doesn't match explicit nulls
        if matches!(input, PipelineData::Empty) {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        input.map(
            move |value| super::operate(&parse, &args, value, head),
            working_set.permanent().ctrlc.clone(),
        )
    }

    #[cfg(windows)]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Parse a single path",
                example: r"'C:\Users\viking\spam.txt' | path parse",
                result: Some(Value::test_record(Record {
                    cols: vec![
                        "prefix".into(),
                        "parent".into(),
                        "stem".into(),
                        "extension".into(),
                    ],
                    vals: vec![
                        Value::test_string("C:"),
                        Value::test_string(r"C:\Users\viking"),
                        Value::test_string("spam"),
                        Value::test_string("txt"),
                    ],
                })),
            },
            Example {
                description: "Replace a complex extension",
                example: r"'C:\Users\viking\spam.tar.gz' | path parse -e tar.gz | upsert extension { 'txt' }",
                result: None,
            },
            Example {
                description: "Ignore the extension",
                example: r"'C:\Users\viking.d' | path parse -e ''",
                result: Some(Value::test_record(Record {
                    cols: vec![
                        "prefix".into(),
                        "parent".into(),
                        "stem".into(),
                        "extension".into(),
                    ],
                    vals: vec![
                        Value::test_string("C:"),
                        Value::test_string(r"C:\Users"),
                        Value::test_string("viking.d"),
                        Value::test_string(""),
                    ],
                })),
            },
            Example {
                description: "Parse all paths in a list",
                example: r"[ C:\Users\viking.d C:\Users\spam.txt ] | path parse",
                result: Some(Value::test_list(vec![
                    Value::test_record(Record {
                        cols: vec![
                            "prefix".into(),
                            "parent".into(),
                            "stem".into(),
                            "extension".into(),
                        ],
                        vals: vec![
                            Value::test_string("C:"),
                            Value::test_string(r"C:\Users"),
                            Value::test_string("viking"),
                            Value::test_string("d"),
                        ],
                    }),
                    Value::test_record(Record {
                        cols: vec![
                            "prefix".into(),
                            "parent".into(),
                            "stem".into(),
                            "extension".into(),
                        ],
                        vals: vec![
                            Value::test_string("C:"),
                            Value::test_string(r"C:\Users"),
                            Value::test_string("spam"),
                            Value::test_string("txt"),
                        ],
                    }),
                ])),
            },
        ]
    }

    #[cfg(not(windows))]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Parse a path",
                example: r"'/home/viking/spam.txt' | path parse",
                result: Some(Value::test_record(Record {
                    cols: vec!["parent".into(), "stem".into(), "extension".into()],
                    vals: vec![
                        Value::test_string("/home/viking"),
                        Value::test_string("spam"),
                        Value::test_string("txt"),
                    ],
                })),
            },
            Example {
                description: "Replace a complex extension",
                example: r"'/home/viking/spam.tar.gz' | path parse -e tar.gz | upsert extension { 'txt' }",
                result: None,
            },
            Example {
                description: "Ignore the extension",
                example: r"'/etc/conf.d' | path parse -e ''",
                result: Some(Value::test_record(Record {
                    cols: vec!["parent".into(), "stem".into(), "extension".into()],
                    vals: vec![
                        Value::test_string("/etc"),
                        Value::test_string("conf.d"),
                        Value::test_string(""),
                    ],
                })),
            },
            Example {
                description: "Parse all paths in a list",
                example: r"[ /home/viking.d /home/spam.txt ] | path parse",
                result: Some(Value::test_list(vec![
                    Value::test_record(Record {
                        cols: vec!["parent".into(), "stem".into(), "extension".into()],
                        vals: vec![
                            Value::test_string("/home"),
                            Value::test_string("viking"),
                            Value::test_string("d"),
                        ],
                    }),
                    Value::test_record(Record {
                        cols: vec!["parent".into(), "stem".into(), "extension".into()],
                        vals: vec![
                            Value::test_string("/home"),
                            Value::test_string("spam"),
                            Value::test_string("txt"),
                        ],
                    }),
                ])),
            },
        ]
    }
}

fn parse(path: &Path, span: Span, args: &Arguments) -> Value {
    let mut record = Record::new();

    #[cfg(windows)]
    {
        use std::path::Component;

        let prefix = match path.components().next() {
            Some(Component::Prefix(prefix_component)) => {
                prefix_component.as_os_str().to_string_lossy()
            }
            _ => "".into(),
        };
        record.push("prefix", Value::string(prefix, span));
    }

    let parent = path
        .parent()
        .unwrap_or_else(|| "".as_ref())
        .to_string_lossy();

    record.push("parent", Value::string(parent, span));

    let basename = path
        .file_name()
        .unwrap_or_else(|| "".as_ref())
        .to_string_lossy();

    match &args.extension {
        Some(Spanned {
            item: extension,
            span: extension_span,
        }) => {
            let ext_with_dot = [".", extension].concat();
            if basename.ends_with(&ext_with_dot) && !extension.is_empty() {
                let stem = basename.trim_end_matches(&ext_with_dot);
                record.push("stem", Value::string(stem, span));
                record.push("extension", Value::string(extension, *extension_span));
            } else {
                record.push("stem", Value::string(basename, span));
                record.push("extension", Value::string("", span));
            }
        }
        None => {
            let stem = path
                .file_stem()
                .unwrap_or_else(|| "".as_ref())
                .to_string_lossy();
            let extension = path
                .extension()
                .unwrap_or_else(|| "".as_ref())
                .to_string_lossy();

            record.push("stem", Value::string(stem, span));
            record.push("extension", Value::string(extension, span));
        }
    }

    Value::record(record, span)
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
