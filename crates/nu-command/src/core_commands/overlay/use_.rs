use nu_engine::{eval_block, find_in_dirs_env, redirect_env, CallExt};
use nu_parser::trim_quotes_str;
use nu_protocol::ast::{Call, Expr};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape, Value,
};

use std::path::Path;

#[derive(Clone)]
pub struct OverlayUse;

impl Command for OverlayUse {
    fn name(&self) -> &str {
        "overlay use"
    }

    fn usage(&self) -> &str {
        "Use definitions from a module as an overlay"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("overlay use")
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

        let origin_module_id = if let Some(overlay_expr) = call.positional_nth(0) {
            if let Expr::Overlay(module_id) = overlay_expr.expr {
                module_id
            } else {
                return Err(ShellError::NushellFailedSpanned(
                    "Not an overlay".to_string(),
                    "requires an overlay (path or a string)".to_string(),
                    overlay_expr.span,
                ));
            }
        } else {
            return Err(ShellError::NushellFailedSpanned(
                "Missing positional".to_string(),
                "missing required overlay".to_string(),
                call.head,
            ));
        };

        let overlay_name = if let Some(kw_expression) = call.positional_nth(1) {
            // If renamed via the 'as' keyword, use the new name as the overlay name
            if let Some(new_name_expression) = kw_expression.as_keyword() {
                if let Some(new_name) = new_name_expression.as_string() {
                    new_name
                } else {
                    return Err(ShellError::NushellFailedSpanned(
                        "Wrong keyword type".to_string(),
                        "keyword argument not a string".to_string(),
                        new_name_expression.span,
                    ));
                }
            } else {
                return Err(ShellError::NushellFailedSpanned(
                    "Wrong keyword type".to_string(),
                    "keyword argument not a keyword".to_string(),
                    kw_expression.span,
                ));
            }
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
            return Err(ShellError::OverlayNotFoundAtRuntime(
                name_arg.item,
                name_arg.span,
            ));
        };

        caller_stack.add_overlay(overlay_name);

        if let Some(module_id) = origin_module_id {
            // Add environment variables only if:
            // a) adding a new overlay
            // b) refreshing an active overlay (the origin module changed)

            let module = engine_state.get_module(module_id);

            // Evaluate the export-env block (if any) and keep its environment
            if let Some(block_id) = module.env_block {
                let maybe_path = find_in_dirs_env(&name_arg.item, engine_state, caller_stack)?;

                if let Some(path) = &maybe_path {
                    // Set the currently evaluated directory, if the argument is a valid path
                    let mut parent = path.clone();
                    parent.pop();

                    let file_pwd = Value::String {
                        val: parent.to_string_lossy().to_string(),
                        span: call.head,
                    };

                    caller_stack.add_env_var("FILE_PWD".to_string(), file_pwd);
                }

                let block = engine_state.get_block(block_id);
                let mut callee_stack = caller_stack.gather_captures(&block.captures);

                let _ = eval_block(
                    engine_state,
                    &mut callee_stack,
                    block,
                    input,
                    call.redirect_stdout,
                    call.redirect_stderr,
                );

                // Merge the block's environment to the current stack
                redirect_env(engine_state, caller_stack, &callee_stack);

                if maybe_path.is_some() {
                    // Remove the file-relative PWD, if the argument is a valid path
                    caller_stack.remove_env_var(engine_state, "FILE_PWD");
                }
            }
        }

        Ok(PipelineData::new(call.head))
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
                example: r#"'export-env { let-env FOO = "foo" }' | save spam.nu
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
