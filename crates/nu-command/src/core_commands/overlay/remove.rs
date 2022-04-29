use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, Signature, Spanned, SyntaxShape};

#[derive(Clone)]
pub struct OverlayRemove;

impl Command for OverlayRemove {
    fn name(&self) -> &str {
        "overlay remove"
    }

    fn usage(&self) -> &str {
        "Remove an active overlay"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("overlay remove")
            .required("name", SyntaxShape::String, "Overlay to remove")
            .category(Category::Core)
    }

    fn extra_usage(&self) -> &str {
        r#"This command is a parser keyword. For details, check
https://www.nushell.sh/book/thinking_in_nushell.html#parsing-and-evaluation-are-different-stages"#
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
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let module_name: Spanned<String> = call.req(engine_state, stack, 0)?;

        stack.remove_overlay(&module_name.item, &module_name.span)?;

        Ok(PipelineData::new(call.head))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            // Example {
            //     description: "Hide the alias just defined",
            //     example: r#"alias lll = ls -l; hide lll"#,
            //     result: None,
            // },
            // Example {
            //     description: "Hide a custom command",
            //     example: r#"def say-hi [] { echo 'Hi!' }; hide say-hi"#,
            //     result: None,
            // },
            // Example {
            //     description: "Hide an environment variable",
            //     example: r#"let-env HZ_ENV_ABC = 1; hide HZ_ENV_ABC; 'HZ_ENV_ABC' in (env).name"#,
            //     result: Some(Value::boolean(false, Span::test_data())),
            // },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(OverlayRemove {})
    }
}
