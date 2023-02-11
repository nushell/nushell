use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, CellPath},
    engine::Command,
    engine::EngineState,
    engine::Stack,
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Type,
    Value,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
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
                (Type::Table(vec![]), Type::Table(vec![])),
                (Type::Record(vec![]), Type::Record(vec![])),
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
                "for a data structure input, add links to all strings at the given cell paths",
            )
            .vectorizes_over_list(true)
            .allow_variants_without_examples(true)
            .category(Category::Platform)
    }

    fn usage(&self) -> &str {
        "Add a link (using OSC 8 escape sequence) to the given string"
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

    fn examples(&self) -> Vec<Example> {
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
            move |v| process_value(&v, &text, &command_span),
            engine_state.ctrlc.clone(),
        )
    } else {
        input.map(
            move |v| process_each_path(v, &column_paths, &text, &command_span),
            engine_state.ctrlc.clone(),
        )
    }
}

fn process_each_path(
    mut value: Value,
    column_paths: &Vec<CellPath>,
    text: &Option<String>,
    command_span: &Span,
) -> Value {
    for path in column_paths {
        let ret = value.update_cell_path(
            &path.members,
            Box::new(|v| process_value(v, text, command_span)),
        );
        if let Err(error) = ret {
            return Value::Error { error };
        }
    }
    value
}

fn process_value(value: &Value, text: &Option<String>, command_span: &Span) -> Value {
    match value {
        Value::String { val, span } => {
            let text = text.as_deref().unwrap_or(val.as_str());
            let result = add_osc_link(text, val.as_str());
            Value::string(result, *span)
        }
        other => {
            let got = format!("value is {}, not string", other.get_type());

            Value::Error {
                error: ShellError::TypeMismatch(got, other.span().unwrap_or(*command_span)),
            }
        }
    }
}

fn add_osc_link(text: &str, link: &str) -> String {
    format!("\u{1b}]8;;{link}\u{1b}\\{text}\u{1b}]8;;\u{1b}\\")
}

#[cfg(test)]
mod tests {
    use super::SubCommand;

    #[test]
    fn examples_work_as_expected() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
