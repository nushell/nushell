use nu_engine::{eval_block, CallExt};
use nu_protocol::ast::{Call, Expr, Expression, ImportPatternMember};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct OverlayAdd;

impl Command for OverlayAdd {
    fn name(&self) -> &str {
        "overlay add"
    }

    fn usage(&self) -> &str {
        "Add definitions from a module as an overlay"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("overlay add")
            .required(
                "name",
                SyntaxShape::String,
                "Module name to create overlay for",
            )
            // .switch(
            //     "prefix",
            //     "Prepend module name to the imported symbols",
            //     Some('p'),
            // )
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
    ) -> Result<PipelineData, ShellError> {
        let module_name: Spanned<String> = call.req(engine_state, stack, 0)?;

        if let Some(module_id) = engine_state.find_module(module_name.item.as_bytes()) {
            let module = engine_state.get_module(module_id);

            stack.add_overlay(module_name.item);

            for (name, block_id) in module.env_vars() {
                let name = if let Ok(s) = String::from_utf8(name.clone()) {
                    s
                } else {
                    return Err(ShellError::NonUtf8(module_name.span));
                };

                let block = engine_state.get_block(block_id);

                let val = eval_block(
                    engine_state,
                    stack,
                    block,
                    PipelineData::new(call.head),
                    false,
                    true,
                )?
                .into_value(call.head);

                stack.add_env_var(name, val);
            }
        } else {
            return Err(ShellError::ModuleNotFoundAtRuntime(
                module_name.item,
                module_name.span,
            ));
        }

        Ok(PipelineData::new(call.head))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            // Example {
            //     description: "Define a custom command in a module and call it",
            //     example: r#"module spam { export def foo [] { "foo" } }; use spam foo; foo"#,
            //     result: Some(Value::String {
            //         val: "foo".to_string(),
            //         span: Span::test_data(),
            //     }),
            // },
            // Example {
            //     description: "Define an environment variable in a module and evaluate it",
            //     example: r#"module foo { export env FOO_ENV { "BAZ" } }; use foo FOO_ENV; $env.FOO_ENV"#,
            //     result: Some(Value::String {
            //         val: "BAZ".to_string(),
            //         span: Span::test_data(),
            //     }),
            // },
            // Example {
            //     description: "Define a custom command that participates in the environment in a module and call it",
            //     example: r#"module foo { export def-env bar [] { let-env FOO_BAR = "BAZ" } }; use foo bar; bar; $env.FOO_BAR"#,
            //     result: Some(Value::String {
            //         val: "BAZ".to_string(),
            //         span: Span::test_data(),
            //     }),
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

        test_examples(OverlayAdd {})
    }
}
