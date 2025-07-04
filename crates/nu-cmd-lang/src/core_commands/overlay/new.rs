use nu_engine::{command_prelude::*, redirect_env};
use nu_protocol::engine::CommandType;

#[derive(Clone)]
pub struct OverlayNew;

impl Command for OverlayNew {
    fn name(&self) -> &str {
        "overlay new"
    }

    fn description(&self) -> &str {
        "Create an empty overlay."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("overlay new")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .allow_variants_without_examples(true)
            .required("name", SyntaxShape::String, "Name of the overlay.")
            .switch(
                "reload",
                "If the overlay already exists, reload its environment.",
                Some('r'),
            )
            // TODO:
            // .switch(
            //     "prefix",
            //     "Prepend module name to the imported symbols",
            //     Some('p'),
            // )
            .category(Category::Core)
    }

    fn extra_description(&self) -> &str {
        r#"The command will first create an empty module, then add it as an overlay.

This command is a parser keyword. For details, check:
  https://www.nushell.sh/book/thinking_in_nu.html"#
    }

    fn command_type(&self) -> CommandType {
        CommandType::Keyword
    }

    fn run(
        &self,
        engine_state: &EngineState,
        caller_stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let name_arg: Spanned<String> = call.req(engine_state, caller_stack, 0)?;
        let reload = call.has_flag(engine_state, caller_stack, "reload")?;

        if reload {
            let callee_stack = caller_stack.clone();
            caller_stack.add_overlay(name_arg.item);
            redirect_env(engine_state, caller_stack, &callee_stack);
        } else {
            caller_stack.add_overlay(name_arg.item);
        }

        Ok(PipelineData::empty())
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
