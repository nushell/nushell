use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct AnsiLink;

impl Command for AnsiLink {
    fn name(&self) -> &str {
        "ansi link"
    }

    fn signature(&self) -> Signature {
        Signature::build("ansi link")
            .input_output_types(vec![
                (Type::String, Type::String),
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::String)),
                ),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .named(
                "text",
                SyntaxShape::String,
                "Link text. Uses uri as text if absent. In case of
                tables, records and lists applies this text to all elements",
                Some('t'),
            )
            .rest(
                "cell path",
                SyntaxShape::CellPath,
                "For a data structure input, add links to all strings at the given cell paths.",
            )
            .allow_variants_without_examples(true)
            .category(Category::Platform)
    }

    fn description(&self) -> &str {
        "Add a link (using OSC 8 escape sequence) to the given string."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        operate(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Create a link to open some file",
                example: "'file:///file.txt' | ansi link --text 'Open Me!'",
                result: Some(Value::string(
                    "\u{1b}]8;;file:///file.txt\u{1b}\\Open Me!\u{1b}]8;;\u{1b}\\",
                    Span::unknown(),
                )),
            },
            Example {
                description: "Create a link without text",
                example: "'https://www.nushell.sh/' | ansi link",
                result: Some(Value::string(
                    "\u{1b}]8;;https://www.nushell.sh/\u{1b}\\https://www.nushell.sh/\u{1b}]8;;\u{1b}\\",
                    Span::unknown(),
                )),
            },
            Example {
                description: "Format a table column into links",
                example: "[[url text]; [https://example.com Text]] | ansi link url",
                result: None,
            },
        ]
    }
}

fn operate(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let text: Option<Spanned<String>> = call.get_flag(engine_state, stack, "text")?;
    let text = text.map(|e| e.item);
    let column_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;

    let command_span = call.head;

    if column_paths.is_empty() {
        input.map(
            move |v| process_value(&v, text.as_deref()),
            engine_state.signals(),
        )
    } else {
        input.map(
            move |v| process_each_path(v, &column_paths, text.as_deref(), command_span),
            engine_state.signals(),
        )
    }
}

fn process_each_path(
    mut value: Value,
    column_paths: &[CellPath],
    text: Option<&str>,
    command_span: Span,
) -> Value {
    for path in column_paths {
        let ret = value.update_cell_path(&path.members, Box::new(|v| process_value(v, text)));
        if let Err(error) = ret {
            return Value::error(error, command_span);
        }
    }
    value
}

fn process_value(value: &Value, text: Option<&str>) -> Value {
    let span = value.span();
    match value {
        Value::String { val, .. } => {
            let text = text.unwrap_or(val.as_str());
            let result = add_osc_link(text, val.as_str());
            Value::string(result, span)
        }
        other => {
            let got = format!("value is {}, not string", other.get_type());

            Value::error(
                ShellError::TypeMismatch {
                    err_message: got,
                    span: other.span(),
                },
                other.span(),
            )
        }
    }
}

fn add_osc_link(text: &str, link: &str) -> String {
    format!("\u{1b}]8;;{link}\u{1b}\\{text}\u{1b}]8;;\u{1b}\\")
}

#[cfg(test)]
mod tests {
    use super::AnsiLink;

    #[test]
    fn examples_work_as_expected() {
        use crate::test_examples;

        test_examples(AnsiLink {})
    }
}
