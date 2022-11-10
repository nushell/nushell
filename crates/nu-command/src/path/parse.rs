use std::path::Path;

use indexmap::IndexMap;
use nu_engine::CallExt;
use nu_protocol::{
    engine::Command, Example, ShellError, Signature, Span, Spanned, SyntaxShape, Type, Value,
};

use super::PathSubcommandArguments;

struct Arguments {
    columns: Option<Vec<String>>,
    extension: Option<Spanned<String>>,
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
        "path parse"
    }

    fn signature(&self) -> Signature {
        Signature::build("path parse")
            .input_output_types(vec![(Type::String, Type::Record(vec![]))])
            .named(
                "columns",
                SyntaxShape::Table,
                "For a record or table input, convert strings at the given columns",
                Some('c'),
            )
            .named(
                "extension",
                SyntaxShape::String,
                "Manually supply the extension (without the dot)",
                Some('e'),
            )
    }

    fn usage(&self) -> &str {
        "Convert a path into structured data."
    }

    fn extra_usage(&self) -> &str {
        r#"Each path is split into a table with 'parent', 'stem' and 'extension' fields.
On Windows, an extra 'prefix' column is added."#
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
            extension: call.get_flag(engine_state, stack, "extension")?,
        };

        input.map(
            move |value| super::operate(&parse, &args, value, head),
            engine_state.ctrlc.clone(),
        )
    }

    #[cfg(windows)]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Parse a single path",
                example: r"'C:\Users\viking\spam.txt' | path parse",
                result: Some(Value::Record {
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
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Replace a complex extension",
                example: r"'C:\Users\viking\spam.tar.gz' | path parse -e tar.gz | upsert extension { 'txt' }",
                result: None,
            },
            Example {
                description: "Ignore the extension",
                example: r"'C:\Users\viking.d' | path parse -e ''",
                result: Some(Value::Record {
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
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Parse all paths under the 'name' column",
                example: r"ls | path parse -c [ name ]",
                result: None,
            },
        ]
    }

    #[cfg(not(windows))]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Parse a path",
                example: r"'/home/viking/spam.txt' | path parse",
                result: Some(Value::Record {
                    cols: vec!["parent".into(), "stem".into(), "extension".into()],
                    vals: vec![
                        Value::test_string("/home/viking"),
                        Value::test_string("spam"),
                        Value::test_string("txt"),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Replace a complex extension",
                example: r"'/home/viking/spam.tar.gz' | path parse -e tar.gz | upsert extension { 'txt' }",
                result: None,
            },
            Example {
                description: "Ignore the extension",
                example: r"'/etc/conf.d' | path parse -e ''",
                result: Some(Value::Record {
                    cols: vec!["parent".into(), "stem".into(), "extension".into()],
                    vals: vec![
                        Value::test_string("/etc"),
                        Value::test_string("conf.d"),
                        Value::test_string(""),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Parse all paths under the 'name' column",
                example: r"ls | path parse -c [ name ]",
                result: None,
            },
        ]
    }
}

fn parse(path: &Path, span: Span, args: &Arguments) -> Value {
    let mut map: IndexMap<String, Value> = IndexMap::new();

    #[cfg(windows)]
    {
        use std::path::Component;

        let prefix = match path.components().next() {
            Some(Component::Prefix(prefix_component)) => {
                prefix_component.as_os_str().to_string_lossy()
            }
            _ => "".into(),
        };
        map.insert("prefix".into(), Value::string(prefix, span));
    }

    let parent = path
        .parent()
        .unwrap_or_else(|| "".as_ref())
        .to_string_lossy();

    map.insert("parent".into(), Value::string(parent, span));

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
                map.insert("stem".into(), Value::string(stem, span));
                map.insert(
                    "extension".into(),
                    Value::string(extension, *extension_span),
                );
            } else {
                map.insert("stem".into(), Value::string(basename, span));
                map.insert("extension".into(), Value::string("", span));
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

            map.insert("stem".into(), Value::string(stem, span));
            map.insert("extension".into(), Value::string(extension, span));
        }
    }

    Value::from(Spanned { item: map, span })
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
