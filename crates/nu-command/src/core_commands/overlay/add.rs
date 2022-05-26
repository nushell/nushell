use nu_engine::{eval_block, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape};

use std::path::Path;

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
            // TODO:
            // .switch(
            //     "prefix",
            //     "Prepend module name to the imported symbols",
            //     Some('p'),
            // )
            .category(Category::Core)
    }

    fn extra_usage(&self) -> &str {
        r#"This command is a parser keyword. For details, check:
  https://www.nushell.sh/book/thinking_in_nushell.html"#
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

        let maybe_overlay_name = if engine_state
            .find_overlay(name_arg.item.as_bytes())
            .is_some()
        {
            Some(name_arg.item.clone())
        } else if let Some(os_str) = Path::new(&name_arg.item).file_stem() {
            os_str.to_str().map(|name| name.to_string())
        } else {
            None
        };

        if let Some(overlay_name) = maybe_overlay_name {
            if let Some(overlay_id) = engine_state.find_overlay(overlay_name.as_bytes()) {
                let old_module_id = engine_state.get_overlay(overlay_id).origin;

                stack.add_overlay(overlay_name.clone());

                if let Some(new_module_id) = engine_state.find_module(overlay_name.as_bytes(), &[])
                {
                    if !stack.has_env_overlay(&overlay_name, engine_state)
                        || (old_module_id != new_module_id)
                    {
                        // Add environment variables only if:
                        // a) adding a new overlay
                        // b) refreshing an active overlay (the origin module changed)
                        let module = engine_state.get_module(new_module_id);

                        for (name, block_id) in module.env_vars() {
                            let name = if let Ok(s) = String::from_utf8(name.clone()) {
                                s
                            } else {
                                return Err(ShellError::NonUtf8(call.head));
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
                    }
                }
            } else {
                return Err(ShellError::OverlayNotFoundAtRuntime(
                    name_arg.item,
                    name_arg.span,
                ));
            }
        } else {
            return Err(ShellError::OverlayNotFoundAtRuntime(
                name_arg.item,
                name_arg.span,
            ));
        }

        Ok(PipelineData::new(call.head))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Create an overlay from a module",
                example: r#"module spam { export def foo [] { "foo" } }
    overlay add spam"#,
                result: None,
            },
            Example {
                description: "Create an overlay from a file",
                example: r#"echo 'export env FOO { "foo" }' | save spam.nu
    overlay add spam.nu"#,
                result: None,
            },
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
