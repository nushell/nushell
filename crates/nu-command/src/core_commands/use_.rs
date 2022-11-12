use nu_engine::{eval_block, find_in_dirs_env, redirect_env};
use nu_protocol::ast::{Call, Expr, Expression};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Use;

impl Command for Use {
    fn name(&self) -> &str {
        "use"
    }

    fn usage(&self) -> &str {
        "Use definitions from a module"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("use")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required("pattern", SyntaxShape::ImportPattern, "import pattern")
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
        let import_pattern = if let Some(Expression {
            expr: Expr::ImportPattern(pat),
            ..
        }) = call.positional_nth(0)
        {
            pat
        } else {
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
                    engine_state.get_span_contents(&import_pattern.head.span),
                );
                let maybe_parent = if let Some(path) =
                    find_in_dirs_env(&module_arg_str, engine_state, caller_stack)?
                {
                    path.parent().map(|p| p.to_path_buf()).or(None)
                } else {
                    None
                };

                let mut callee_stack = caller_stack.gather_captures(&block.captures);

                // If so, set the currently evaluated directory (file-relative PWD)
                if let Some(parent) = maybe_parent {
                    let file_pwd = Value::String {
                        val: parent.to_string_lossy().to_string(),
                        span: call.head,
                    };
                    callee_stack.add_env_var("FILE_PWD".to_string(), file_pwd);
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

        Ok(PipelineData::new(call.head))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Define a custom command in a module and call it",
                example: r#"module spam { export def foo [] { "foo" } }; use spam foo; foo"#,
                result: Some(Value::String {
                    val: "foo".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Define a custom command that participates in the environment in a module and call it",
                example: r#"module foo { export def-env bar [] { let-env FOO_BAR = "BAZ" } }; use foo bar; bar; $env.FOO_BAR"#,
                result: Some(Value::String {
                    val: "BAZ".to_string(),
                    span: Span::test_data(),
                }),
            },
        ]
    }
}
