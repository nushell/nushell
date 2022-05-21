use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, Signature, Spanned, SyntaxShape, ShellError};

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
            .optional("name", SyntaxShape::String, "Overlay to remove")
            .switch(
                "keep",
                "Keep newly added symbols within the next activated overlay",
                Some('k'),
            )
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
        let module_name: Spanned<String> = if let Some(name) = call.opt(engine_state, stack, 0)? {
            name
        } else {
            Spanned {
                item: stack.last_overlay_name()?,
                span: call.head,
            }
        };

        let env_vars_to_keep = if call.has_flag("keep") {
            if let Some(id) = engine_state.find_active_overlay(module_name.item.as_bytes()) {
                let overlay_frame = engine_state.get_overlay(id);
                let orig_module = engine_state.get_module(overlay_frame.origin);

                let mut result = stack.get_overlay_env_vars(engine_state, &module_name.item);
                result.retain(|name, _| !orig_module.has_env_var(name.as_bytes()));

                Some(result)
            } else {
                if let Some(id) = engine_state.find_module(module_name.item.as_bytes(), &[]) {
                    let orig_module = engine_state.get_module(id);

                    let mut result = stack.get_overlay_env_vars(engine_state, &module_name.item);
                    result.retain(|name, _| !orig_module.has_env_var(name.as_bytes()));

                    Some(result)
                } else {
                    return Err(ShellError::ModuleOrOverlayNotFoundAtRuntime(
                        module_name.item,
                        module_name.span,
                    ));
                }
            }
        } else {
            None
        };

        stack.remove_overlay(&module_name.item, &module_name.span)?;

        if let Some(env_vars) = env_vars_to_keep {
            for (name, val) in env_vars.into_iter() {
                stack.add_env_var(name, val);
            }
        }

        Ok(PipelineData::new(call.head))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Remove an overlay created from a module",
                example: r#"module spam { export def foo [] { "foo" } }
    overlay add spam
    overlay remove spam"#,
                result: None,
            },
            Example {
                description: "Remove an overlay created from a file",
                example: r#"echo 'export alias f = "foo"' | save spam.nu
    overlay add spam.nu
    overlay remove spam"#,
                result: None,
            },
            Example {
                description: "Remove the last activated overlay",
                example: r#"module spam { export env FOO { "foo" } }
    overlay add spam
    overlay remove"#,
                result: None,
            },
        ]
    }
}
