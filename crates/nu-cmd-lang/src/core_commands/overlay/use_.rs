use nu_engine::{
    command_prelude::*, find_in_dirs_env, get_dirs_var_from_call, get_eval_block, redirect_env,
};
use nu_parser::trim_quotes_str;
use nu_protocol::{ModuleId, ast::Expr, engine::CommandType};

use std::path::Path;

#[derive(Clone)]
pub struct OverlayUse;

impl Command for OverlayUse {
    fn name(&self) -> &str {
        "overlay use"
    }

    fn description(&self) -> &str {
        "Use definitions from a module as an overlay."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("overlay use")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .allow_variants_without_examples(true)
            .required(
                "name",
                SyntaxShape::OneOf(vec![SyntaxShape::String, SyntaxShape::Nothing]),
                "Module name to use overlay for (`null` for no-op).",
            )
            .optional(
                "as",
                SyntaxShape::Keyword(b"as".to_vec(), Box::new(SyntaxShape::String)),
                "`as` keyword followed by a new name.",
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

    fn extra_description(&self) -> &str {
        r#"This command is a parser keyword. For details, check:
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
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let noop = call.get_parser_info(caller_stack, "noop");
        if noop.is_some() {
            return Ok(PipelineData::empty());
        }

        let name_arg: Spanned<String> = call.req(engine_state, caller_stack, 0)?;
        let name_arg_item = trim_quotes_str(&name_arg.item);

        let maybe_origin_module_id: Option<ModuleId> =
            if let Some(overlay_expr) = call.get_parser_info(caller_stack, "overlay_expr") {
                if let Expr::Overlay(module_id) = &overlay_expr.expr {
                    *module_id
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
            .find_overlay(name_arg_item.as_bytes())
            .is_some()
        {
            name_arg_item.to_string()
        } else if let Some(os_str) = Path::new(name_arg_item).file_stem() {
            if let Some(name) = os_str.to_str() {
                name.to_string()
            } else {
                return Err(ShellError::NonUtf8 {
                    span: name_arg.span,
                });
            }
        } else {
            return Err(ShellError::OverlayNotFoundAtRuntime {
                overlay_name: (name_arg_item.to_string()),
                span: name_arg.span,
            });
        };

        if let Some(module_id) = maybe_origin_module_id {
            // Add environment variables only if (determined by parser):
            // a) adding a new overlay
            // b) refreshing an active overlay (the origin module changed)

            let module = engine_state.get_module(module_id);
            // in such case, should also make sure that PWD is not restored in old overlays.
            let cwd = caller_stack.get_env_var(engine_state, "PWD").cloned();

            // Evaluate the export-env block (if any) and keep its environment
            if let Some(block_id) = module.env_block {
                let maybe_file_path_or_dir = find_in_dirs_env(
                    name_arg_item,
                    engine_state,
                    caller_stack,
                    get_dirs_var_from_call(caller_stack, call),
                )?;
                let block = engine_state.get_block(block_id);
                let mut callee_stack = caller_stack
                    .gather_captures(engine_state, &block.captures)
                    .reset_pipes();

                if let Some(path) = &maybe_file_path_or_dir {
                    // Set the currently evaluated directory, if the argument is a valid path
                    let parent = if path.is_dir() {
                        path.clone()
                    } else {
                        let mut parent = path.clone();
                        parent.pop();
                        parent
                    };
                    let file_pwd = Value::string(parent.to_string_lossy(), call.head);

                    callee_stack.add_env_var("FILE_PWD".to_string(), file_pwd);
                }

                if let Some(path) = &maybe_file_path_or_dir {
                    let module_file_path = if path.is_dir() {
                        // the existence of `mod.nu` is verified in parsing time
                        // so it's safe to use it here.
                        Value::string(path.join("mod.nu").to_string_lossy(), call.head)
                    } else {
                        Value::string(path.to_string_lossy(), call.head)
                    };
                    callee_stack.add_env_var("CURRENT_FILE".to_string(), module_file_path);
                }

                let eval_block = get_eval_block(engine_state);
                let _ = eval_block(engine_state, &mut callee_stack, block, input)?;

                // The export-env block should see the env vars *before* activating this overlay
                caller_stack.add_overlay(overlay_name);
                // make sure that PWD is not restored in old overlays.
                if let Some(cwd) = cwd {
                    caller_stack.add_env_var("PWD".to_string(), cwd);
                }

                // Merge the block's environment to the current stack
                redirect_env(engine_state, caller_stack, &callee_stack);
            } else {
                caller_stack.add_overlay(overlay_name);
                // make sure that PWD is not restored in old overlays.
                if let Some(cwd) = cwd {
                    caller_stack.add_env_var("PWD".to_string(), cwd);
                }
            }
        } else {
            caller_stack.add_overlay(overlay_name);
            caller_stack.update_config(engine_state)?;
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
