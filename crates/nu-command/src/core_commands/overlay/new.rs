use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape};

#[derive(Clone)]
pub struct OverlayNew;

impl Command for OverlayNew {
    fn name(&self) -> &str {
        "overlay new"
    }

    fn usage(&self) -> &str {
        "Create an empty overlay"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("overlay new")
            .required("name", SyntaxShape::String, "Name of the overlay")
            // TODO:
            // .switch(
            //     "prefix",
            //     "Prepend module name to the imported symbols",
            //     Some('p'),
            // )
            .category(Category::Core)
    }

    fn extra_usage(&self) -> &str {
        r#"The command will first create an empty module, then add it as an overlay.

This command is a parser keyword. For details, check:
  https://www.nushell.sh/book/thinking_in_nu.html"#
    }

    fn is_parser_keyword(&self) -> bool {
        true
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let name_arg: Spanned<String> = call.req(engine_state, stack, 0)?;

        stack.add_overlay(name_arg.item);

        Ok(PipelineData::new(call.head))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Create an empty overlay",
            example: r#"overlay new spam"#,
            result: None,
        }]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(OverlayNew {})
    }
}
