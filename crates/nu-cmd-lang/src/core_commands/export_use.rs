use nu_engine::{
    command_prelude::*, find_in_dirs_env, get_dirs_var_from_call, get_eval_block, redirect_env,
};
use nu_protocol::{
    ast::{Expr, Expression},
    engine::CommandType,
};

#[derive(Clone)]
pub struct ExportUse;

impl Command for ExportUse {
    fn name(&self) -> &str {
        "export use"
    }

    fn description(&self) -> &str {
        "Use definitions from a module and export them from this module."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("export use")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required("module", SyntaxShape::String, "Module or module file.")
            .rest(
                "members",
                SyntaxShape::Any,
                "Which members of the module to import.",
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
        if call.get_parser_info(caller_stack, "noop").is_some() {
            return Ok(PipelineData::empty());
        }
        let Some(Expression {
            expr: Expr::ImportPattern(import_pattern),
            ..
        }) = call.get_parser_info(caller_stack, "import_pattern")
        else {
            return Err(ShellError::GenericError {
                error: "Unexpected import".into(),
                msg: "import pattern not supported".into(),
                span: Some(call.head),
                help: None,
                inner: vec![],
            });
        };

        // Necessary so that we can modify the stack.
        let import_pattern = import_pattern.clone();

        if let Some(module_id) = import_pattern.head.id {
            // Add constants
            for var_id in &import_pattern.constants {
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

            // Evaluate the export-env block if there is one
            let module = engine_state.get_module(module_id);

            if let Some(block_id) = module.env_block {
                let block = engine_state.get_block(block_id);

                // See if the module is a file
                let module_arg_str = String::from_utf8_lossy(
                    engine_state.get_span_contents(import_pattern.head.span),
                );

                let maybe_file_path_or_dir = find_in_dirs_env(
                    &module_arg_str,
                    engine_state,
                    caller_stack,
                    get_dirs_var_from_call(caller_stack, call),
                )?;
                // module_arg_str maybe a directory, in this case
                // find_in_dirs_env returns a directory.
                let maybe_parent = maybe_file_path_or_dir.as_ref().and_then(|path| {
                    if path.is_dir() {
                        Some(path.to_path_buf())
                    } else {
                        path.parent().map(|p| p.to_path_buf())
                    }
                });

                let mut callee_stack = caller_stack
                    .gather_captures(engine_state, &block.captures)
                    .reset_pipes();

                // If so, set the currently evaluated directory (file-relative PWD)
                if let Some(parent) = maybe_parent {
                    let file_pwd = Value::string(parent.to_string_lossy(), call.head);
                    callee_stack.add_env_var("FILE_PWD".to_string(), file_pwd);
                }

                if let Some(path) = maybe_file_path_or_dir {
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

                // Run the block (discard the result)
                let _ = eval_block(engine_state, &mut callee_stack, block, input)?;

                // Merge the block's environment to the current stack
                redirect_env(engine_state, caller_stack, &callee_stack);
            }
        } else {
            return Err(ShellError::GenericError {
                error: format!(
                    "Could not import from '{}'",
                    String::from_utf8_lossy(&import_pattern.head.name)
                ),
                msg: "module does not exist".to_string(),
                span: Some(import_pattern.head.span),
                help: None,
                inner: vec![],
            });
        }

        Ok(PipelineData::empty())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Re-export a command from another module",
            example: r#"module spam { export def foo [] { "foo" } }
    module eggs { export use spam foo }
    use eggs foo
    foo
            "#,
            result: Some(Value::test_string("foo")),
        }]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["reexport", "import", "module"]
    }
}
