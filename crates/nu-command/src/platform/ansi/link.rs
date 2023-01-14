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
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        operate(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Create a link to open some file",
                example: "ansi link 'file:///file.txt' 'Open Me!'",
                result: Some(Value::string(
                    "\u{1b}]8;;file:///file.txt\u{1b}\\Open Me!\u{1b}]8;;\u{1b}\\",
                    Span::unknown(),
                )),
            },
            Example {
                description: "Create link to nushell website",
                example: "ansi link 'https://www.nushell.sh/' 'Nushell'",
                result: Some(Value::string(
                    "\u{1b}]8;;https://www.nushell.sh/\u{1b}\\Nushell\u{1b}]8;;\u{1b}\\",
                    Span::unknown(),
                )),
            },
            Example {
                description: "Create a link without text",
                example: "ansi link 'https://www.nushell.sh/'",
                result: Some(Value::string(
                    "\u{1b}]8;;https://www.nushell.sh/\u{1b}\\https://www.nushell.sh/\u{1b}]8;;\u{1b}\\",
                    Span::unknown(),
                )),
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
            move |mut v| {
                for path in &column_paths {
                    let at_path = v.clone().follow_cell_path(&path.members, false);

                    let at_path = match at_path {
                        Err(error) => return Value::Error { error },
                        Ok(val) => val,
                    };

                    let new_val = process_value(&at_path, &text, &command_span);
                    let res = v.update_data_at_cell_path(&path.members, new_val);
                    if let Err(error) = res {
                        return Value::Error { error };
                    }
                }
                v
            },
            engine_state.ctrlc.clone(),
        )
    }
}

fn process_value(value: &Value, text: &Option<String>, command_span: &Span) -> Value {
    match value {
        Value::String { val, span } => {
            let text = text.as_deref().unwrap_or_else(|| val.as_str());
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
    format!("\u{1b}]8;;{}\u{1b}\\{}\u{1b}]8;;\u{1b}\\", link, text)
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
