use nu_engine::eval_block;
use nu_protocol::ast::{Call, Expr, Expression, ImportPatternMember};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
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
            .required("pattern", SyntaxShape::ImportPattern, "import pattern")
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

            let env_vars_to_use = if import_pattern.members.is_empty() {
                module.env_vars_with_head(&import_pattern.head.name)
            } else {
                match &import_pattern.members[0] {
                    ImportPatternMember::Glob { .. } => module.env_vars(),
                    ImportPatternMember::Name { name, span } => {
                        let mut output = vec![];

                        if let Some(id) = module.get_env_var_id(name) {
                            output.push((name.clone(), id));
                        } else if !module.has_decl(name) && !module.has_alias(name) {
                            return Err(ShellError::EnvVarNotFoundAtRuntime(
                                String::from_utf8_lossy(name).into(),
                                *span,
                            ));
                        }

                        output
                    }
                    ImportPatternMember::List { names } => {
                        let mut output = vec![];

                        for (name, span) in names {
                            if let Some(id) = module.get_env_var_id(name) {
                                output.push((name.clone(), id));
                            } else if !module.has_decl(name) && !module.has_alias(name) {
                                return Err(ShellError::EnvVarNotFoundAtRuntime(
                                    String::from_utf8_lossy(name).into(),
                                    *span,
                                ));
                            }
                        }

                        output
                    }
                }
            };

            for (name, block_id) in env_vars_to_use {
                let name = if let Ok(s) = String::from_utf8(name.clone()) {
                    s
                } else {
                    return Err(ShellError::NonUtf8(import_pattern.head.span));
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
            // TODO: This is a workaround since call.positional[0].span points at 0 for some reason
            // when this error is triggered
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
                description: "Define an environment variable in a module and evaluate it",
                example: r#"module foo { export env FOO_ENV { "BAZ" } }; use foo FOO_ENV; $env.FOO_ENV"#,
                result: Some(Value::String {
                    val: "BAZ".to_string(),
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
