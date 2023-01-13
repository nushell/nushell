use nu_engine::CallExt;
use nu_protocol::{
    ast::Call, engine::Command, engine::EngineState, engine::Stack, Category, Example,
    IntoPipelineData, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "ansi link"
    }

    fn signature(&self) -> Signature {
        Signature::build("ansi link")
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .required("uri", SyntaxShape::String, "URI link to embed in text")
            .optional(
                "text",
                SyntaxShape::String,
                "Link text. Uses uri as text if absent",
            )
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
    let uri: Spanned<String> = call.req(engine_state, stack, 0)?;
    let text: Option<Spanned<String>> = call.opt(engine_state, stack, 1)?;

    // If text is absent use provided URI
    let text = text.unwrap_or_else(|| uri.clone());

    let output = add_osc_link(&text.item, &uri.item);
    Ok(Value::string(output, call.head).into_pipeline_data())
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
