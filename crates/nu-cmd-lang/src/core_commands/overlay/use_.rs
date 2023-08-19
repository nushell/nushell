use nu_engine::{eval_block, find_in_dirs_env, get_dirs_var_from_call, redirect_env, CallExt};
use nu_parser::trim_quotes_str;
use nu_protocol::ast::{Call, Expr};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape, Type, Value,
};

use std::path::Path;

#[derive(Clone)]
pub struct OverlayUse;

impl Command for OverlayUse {
    fn name(&self) -> &str {
        "overlay use"
    }

    fn usage(&self) -> &str {
        "Use definitions from a module as an overlay."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("overlay use")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .allow_variants_without_examples(true)
            .required(
                "name",
                SyntaxShape::String,
                "Module name to use overlay for",
            )
            .optional(
                "as",
                SyntaxShape::Keyword(b"as".to_vec(), Box::new(SyntaxShape::String)),
                "as keyword followed by a new name",
            )
            .switch(
                "prefix",
                "Prepend module name to the imported commands and aliases",
                Some('p'),
            )
            .switch(
                "reload",
                "If the overlay already exists, reload its definitions and environment.",
                Some('r'),
            )
            .category(Category::Core)
    }

    fn extra_usage(&self) -> &str {
        r#"This command is a parser keyword. For details, check:
  https://www.nushell.sh/book/thinking_in_nu.html"#
    }

    fn is_parser_keyword(&self) -> bool {
        true
    }

    fn run(
        &self,
        engine_state: &EngineState,
        caller_stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let mut name_arg: Spanned<String> = call.req(engine_state, caller_stack, 0)?;
        name_arg.item = trim_quotes_str(&name_arg.item).to_string();

        let (maybe_origin_module_id, constants) =
            if let Some(overlay_expr) = call.get_parser_info("overlay_expr") {
                if let Expr::Overlay(module_id, constants) = &overlay_expr.expr {
                    (module_id, constants)
                } else {
                    return Err(ShellError::NushellFailedSpanned {
                        msg: "Not an overlay".to_string(),
                        label: "requires an overlay (path or a string)".to_string(),
                        span: overlay_expr.span,
                    });
                }
            } else {
                return Err(ShellError::NushellFailedSpanned {
                    msg: "Missing positional".to_string(),
                    label: "missing required overlay".to_string(),
                    span: call.head,
                });
            };

        let overlay_name = if let Some(name) = call.opt(engine_state, caller_stack, 1)? {
            name
        } else if engine_state
            .find_overlay(name_arg.item.as_bytes())
            .is_some()
        {
            name_arg.item.clone()
        } else if let Some(os_str) = Path::new(&name_arg.item).file_stem() {
            if let Some(name) = os_str.to_str() {
                name.to_string()
            } else {
                return Err(ShellError::NonUtf8(name_arg.span));
            }
        } else {
            return Err(ShellError::OverlayNotFoundAtRuntime {
                overlay_name: name_arg.item,
                span: name_arg.span,
            });
        };

        if let Some(module_id) = maybe_origin_module_id {
            // Add environment variables / constants only if (determined by parser):
            // a) adding a new overlay
            // b) refreshing an active overlay (the origin module changed)

            let module = engine_state.get_module(*module_id);

            // Add constants
            for var_id in constants {
                let var = engine_state.get_var(*var_id);

                if let Some(constval) = &var.const_val {
                    caller_stack.add_var(*var_id, constval.clone());
                } else {
                    return Err(ShellError::NushellFailedSpanned {
                        msg: "Missing Constant".to_string(),
                        label: "constant not added by the parser".to_string(),
                        span: var.declaration_span,
                    });
                }
            }

            // Evaluate the export-env block (if any) and keep its environment
            if let Some(block_id) = module.env_block {
                let maybe_path = find_in_dirs_env(
                    &name_arg.item,
                    engine_state,
                    caller_stack,
                    get_dirs_var_from_call(call),
                )?;

                let block = engine_state.get_block(block_id);
                let mut callee_stack = caller_stack.gather_captures(engine_state, &block.captures);

                if let Some(path) = &maybe_path {
                    // Set the currently evaluated directory, if the argument is a valid path
                    let mut parent = path.clone();
                    parent.pop();

                    let file_pwd = Value::string(parent.to_string_lossy(), call.head);

                    callee_stack.add_env_var("FILE_PWD".to_string(), file_pwd);
                }

                if let Some(file_path) = &maybe_path {
                    let file_path = Value::string(file_path.to_string_lossy(), call.head);
                    callee_stack.add_env_var("CURRENT_FILE".to_string(), file_path);
                }

                let _ = eval_block(
                    engine_state,
                    &mut callee_stack,
                    block,
                    input,
                    call.redirect_stdout,
                    call.redirect_stderr,
                );

                // The export-env block should see the env vars *before* activating this overlay
                caller_stack.add_overlay(overlay_name);

                // Merge the block's environment to the current stack
                redirect_env(engine_state, caller_stack, &callee_stack);
            } else {
                caller_stack.add_overlay(overlay_name);
            }
        } else {
            caller_stack.add_overlay(overlay_name);
        }

        Ok(PipelineData::empty())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Create an overlay from a module",
                example: r#"module spam { export def foo [] { "foo" } }
    overlay use spam
    foo"#,
                result: None,
            },
            Example {
                description: "Create an overlay from a module and rename it",
                example: r#"module spam { export def foo [] { "foo" } }
    overlay use spam as spam_new
    foo"#,
                result: None,
            },
            Example {
                description: "Create an overlay with a prefix",
                example: r#"'export def foo { "foo" }'
    overlay use --prefix spam
    spam foo"#,
                result: None,
            },
            Example {
                description: "Create an overlay from a file",
                example: r#"'export-env { $env.FOO = "foo" }' | save spam.nu
    overlay use spam.nu
    $env.FOO"#,
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

        test_examples(OverlayUse {})
    }
}
