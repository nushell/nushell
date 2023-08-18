use nu_engine::{eval_block, find_in_dirs_env, get_dirs_var_from_call, redirect_env};
use nu_protocol::ast::{Call, Expr, Expression, ImportPattern, ImportPatternMember};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, Module, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Use;

impl Command for Use {
    fn name(&self) -> &str {
        "use"
    }

    fn usage(&self) -> &str {
        "Use definitions from a module, making them available in your shell."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("use")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .allow_variants_without_examples(true)
            .required("module", SyntaxShape::String, "Module or module file")
            .rest(
                "members",
                SyntaxShape::Any,
                "Which members of the module to import",
            )
            .category(Category::Core)
    }

    fn extra_usage(&self) -> &str {
        r#"See `help std` for the standard library module.
See `help modules` to list all available modules.

This command is a parser keyword. For details, check:
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
        let Some(Expression {
            expr: Expr::ImportPattern(import_pattern),
            ..
        }) = call.get_parser_info("import_pattern") else {
            return Err(ShellError::GenericError(
                "Unexpected import".into(),
                "import pattern not supported".into(),
                Some(call.head),
                None,
                Vec::new(),
            ));
        };

        if let Some(module_id) = import_pattern.head.id {
            let module = engine_state.get_module(module_id);

            // Evaluate the export-env block if there is one
            if let Some(block_id) = module.env_block {
                let block = engine_state.get_block(block_id);

                // See if the module is a file
                let module_arg_str = String::from_utf8_lossy(
                    engine_state.get_span_contents(import_pattern.head.span),
                );

                let maybe_file_path = find_in_dirs_env(
                    &module_arg_str,
                    engine_state,
                    caller_stack,
                    get_dirs_var_from_call(call),
                )?;
                let maybe_parent = maybe_file_path
                    .as_ref()
                    .and_then(|path| path.parent().map(|p| p.to_path_buf()));

                let mut callee_stack = caller_stack.gather_captures(&block.captures);

                // If so, set the currently evaluated directory (file-relative PWD)
                if let Some(parent) = maybe_parent {
                    let file_pwd = Value::string(parent.to_string_lossy(), call.head);
                    callee_stack.add_env_var("FILE_PWD".to_string(), file_pwd);
                }

                if let Some(file_path) = maybe_file_path {
                    let file_path = Value::string(file_path.to_string_lossy(), call.head);
                    callee_stack.add_env_var("CURRENT_FILE".to_string(), file_path);
                }

                // Run the block (discard the result)
                let _ = eval_block(
                    engine_state,
                    &mut callee_stack,
                    block,
                    input,
                    call.redirect_stdout,
                    call.redirect_stderr,
                )?;

                // Merge the block's environment to the current stack
                redirect_env(engine_state, caller_stack, &callee_stack);
            }

            use_variables(
                engine_state,
                import_pattern,
                module,
                caller_stack,
                call.head,
            );
        } else {
            return Err(ShellError::GenericError(
                format!(
                    "Could not import from '{}'",
                    String::from_utf8_lossy(&import_pattern.head.name)
                ),
                "module does not exist".to_string(),
                Some(import_pattern.head.span),
                None,
                Vec::new(),
            ));
        }

        Ok(PipelineData::empty())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Define a custom command in a module and call it",
                example: r#"module spam { export def foo [] { "foo" } }; use spam foo; foo"#,
                result: Some(Value::test_string("foo")),
            },
            Example {
                description: "Define a custom command that participates in the environment in a module and call it",
                example: r#"module foo { export def-env bar [] { $env.FOO_BAR = "BAZ" } }; use foo bar; bar; $env.FOO_BAR"#,
                result: Some(Value::test_string("BAZ")),
            },
            Example {
                description: "Use a plain module name to import its definitions qualified by the module name",
                example: r#"module spam { export def foo [] { "foo" }; export def bar [] { "bar" } }; use spam; (spam foo) + (spam bar)"#,
                result: Some(Value::test_string("foobar")),
            },
            Example {
                description: "Specify * to use all definitions in a module",
                example: r#"module spam { export def foo [] { "foo" }; export def bar [] { "bar" } }; use spam *; (foo) + (bar)"#,
                result: Some(Value::test_string("foobar")),
            },
            Example {
                description: "To use commands with spaces, like subcommands, surround them with quotes",
                example: r#"module spam { export def 'foo bar' [] { "baz" } }; use spam 'foo bar'; foo bar"#,
                result: Some(Value::test_string("baz")),
            },
            Example {
                description: "To use multiple definitions from a module, wrap them in a list",
                example: r#"module spam { export def foo [] { "foo" }; export def 'foo bar' [] { "baz" } }; use spam ['foo', 'foo bar']; (foo) + (foo bar)"#,
                result: Some(Value::test_string("foobaz")),
            },
        ]
    }
}

fn use_variables(
    engine_state: &EngineState,
    import_pattern: &ImportPattern,
    module: &Module,
    caller_stack: &mut Stack,
    head_span: Span,
) {
    if !module.variables.is_empty() {
        if import_pattern.members.is_empty() {
            // add a record variable.
            if let Some(var_id) = import_pattern.module_name_var_id {
                let mut cols = vec![];
                let mut vals = vec![];
                for (var_name, var_id) in module.variables.iter() {
                    if let Some(val) = engine_state.get_var(*var_id).clone().const_val {
                        cols.push(String::from_utf8_lossy(var_name).to_string());
                        vals.push(val)
                    }
                }
                caller_stack.add_var(
                    var_id,
                    Value::record(cols, vals, module.span.unwrap_or(head_span)),
                )
            }
        } else {
            let mut have_glob = false;
            for m in &import_pattern.members {
                if matches!(m, ImportPatternMember::Glob { .. }) {
                    have_glob = true;
                    break;
                }
            }
            if have_glob {
                // bring all variables into scope directly.
                for (_, var_id) in module.variables.iter() {
                    if let Some(val) = engine_state.get_var(*var_id).clone().const_val {
                        caller_stack.add_var(*var_id, val);
                    }
                }
            } else {
                let mut members = vec![];
                for m in &import_pattern.members {
                    match m {
                        ImportPatternMember::List { names, .. } => {
                            for (n, _) in names {
                                if module.variables.contains_key(n) {
                                    members.push(n);
                                }
                            }
                        }
                        ImportPatternMember::Name { name, .. } => {
                            if module.variables.contains_key(name) {
                                members.push(name)
                            }
                        }
                        ImportPatternMember::Glob { .. } => continue,
                    }
                }
                for m in members {
                    if let Some(var_id) = module.variables.get(m) {
                        if let Some(val) = engine_state.get_var(*var_id).clone().const_val {
                            caller_stack.add_var(*var_id, val);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() {
        use super::Use;
        use crate::test_examples;
        test_examples(Use {})
    }
}
