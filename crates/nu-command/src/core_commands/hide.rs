use nu_protocol::ast::{Call, Expr, Expression, ImportPatternMember};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Hide;

impl Command for Hide {
    fn name(&self) -> &str {
        "hide"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("hide")
            .required("pattern", SyntaxShape::ImportPattern, "import pattern")
            .category(Category::Core)
    }

    fn usage(&self) -> &str {
        "Hide symbols in the current scope"
    }

    fn extra_usage(&self) -> &str {
        r#"Symbols are hidden by priority: First aliases, then custom commands, then environment variables.

This command is a parser keyword. For details, check:
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
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
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

        let head_name_str = if let Ok(s) = String::from_utf8(import_pattern.head.name.clone()) {
            s
        } else {
            return Err(ShellError::NonUtf8(import_pattern.head.span));
        };

        if let Some(module_id) = engine_state.find_module(&import_pattern.head.name, &[]) {
            // The first word is a module
            let module = engine_state.get_module(module_id);

            let env_vars_to_hide = if import_pattern.members.is_empty() {
                module.env_vars_with_head(&import_pattern.head.name)
            } else {
                match &import_pattern.members[0] {
                    ImportPatternMember::Glob { .. } => module.env_vars(),
                    ImportPatternMember::Name { name, span } => {
                        let mut output = vec![];

                        if let Some((name, id)) =
                            module.env_var_with_head(name, &import_pattern.head.name)
                        {
                            output.push((name, id));
                        } else if !(module.has_alias(name) || module.has_decl(name)) {
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
                            if let Some((name, id)) =
                                module.env_var_with_head(name, &import_pattern.head.name)
                            {
                                output.push((name, id));
                            } else if !(module.has_alias(name) || module.has_decl(name)) {
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

            for (name, _) in env_vars_to_hide {
                let name = if let Ok(s) = String::from_utf8(name.clone()) {
                    s
                } else {
                    return Err(ShellError::NonUtf8(import_pattern.span()));
                };

                if stack.remove_env_var(engine_state, &name).is_none() {
                    return Err(ShellError::NotFound(
                        call.positional_nth(0)
                            .expect("already checked for present positional")
                            .span,
                    ));
                }
            }
        } else if !import_pattern.hidden.contains(&import_pattern.head.name)
            && stack.remove_env_var(engine_state, &head_name_str).is_none()
        {
            // TODO: we may want to error in the future
        }

        Ok(PipelineData::new(call.head))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Hide the alias just defined",
                example: r#"alias lll = ls -l; hide lll"#,
                result: None,
            },
            Example {
                description: "Hide a custom command",
                example: r#"def say-hi [] { echo 'Hi!' }; hide say-hi"#,
                result: None,
            },
            Example {
                description: "Hide an environment variable",
                example: r#"let-env HZ_ENV_ABC = 1; hide HZ_ENV_ABC; 'HZ_ENV_ABC' in (env).name"#,
                result: Some(Value::boolean(false, Span::test_data())),
            },
        ]
    }
}
