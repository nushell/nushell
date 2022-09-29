use itertools::Itertools;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape,
    Value,
};

#[derive(Clone)]
pub struct ViewSource;

impl Command for ViewSource {
    fn name(&self) -> &str {
        "view-source"
    }

    fn usage(&self) -> &str {
        "View a block, module, or a definition"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("view-source")
            .required("item", SyntaxShape::Any, "name or block to view")
            .category(Category::Core)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let arg: Value = call.req(engine_state, stack, 0)?;
        let arg_span = arg.span()?;

        match arg {
            Value::Block { val: block_id, .. } => {
                let block = engine_state.get_block(block_id);

                if let Some(span) = block.span {
                    let contents = engine_state.get_span_contents(&span);
                    Ok(Value::string(String::from_utf8_lossy(contents), call.head)
                        .into_pipeline_data())
                } else {
                    Ok(Value::string("<internal command>", call.head).into_pipeline_data())
                }
            }
            Value::String { val, .. } => {
                if let Some(decl_id) = engine_state.find_decl(val.as_bytes(), &[]) {
                    // arg is a command
                    let decl = engine_state.get_decl(decl_id);
                    let sig = decl.signature();
                    let vec_of_required = &sig.required_positional;
                    let vec_of_optional = &sig.optional_positional;
                    let vec_of_flags = &sig.named;
                    // gets vector of positionals.
                    if let Some(block_id) = decl.get_block_id() {
                        let block = engine_state.get_block(block_id);
                        if let Some(block_span) = block.span {
                            let contents = engine_state.get_span_contents(&block_span);
                            let mut final_contents = String::from("def ");
                            final_contents.push_str(&val);
                            // The name of the function...
                            final_contents.push_str(" [ ");
                            for n in vec_of_required {
                                final_contents.push_str(&n.name);
                                // name of positional arg
                                final_contents.push(':');
                                final_contents.push_str(&n.shape.to_string());
                                final_contents.push(' ');
                            }
                            for n in vec_of_optional {
                                final_contents.push_str(&n.name);
                                // name of positional arg
                                final_contents.push_str("?:");
                                final_contents.push_str(&n.shape.to_string());
                                final_contents.push(' ');
                            }
                            for n in vec_of_flags {
                                final_contents.push_str("--");
                                final_contents.push_str(&n.long);
                                final_contents.push(' ');
                                if n.short.is_some() {
                                    final_contents.push_str("(-");
                                    final_contents.push(n.short.expect("this cannot trigger."));
                                    final_contents.push(')');
                                }
                                if n.arg.is_some() {
                                    final_contents.push_str(": ");
                                    final_contents.push_str(
                                        &n.arg.as_ref().expect("this cannot trigger.").to_string(),
                                    );
                                }
                                final_contents.push(' ');
                            }
                            final_contents.push_str("] ");
                            final_contents.push_str(&String::from_utf8_lossy(contents));
                            Ok(Value::string(final_contents, call.head).into_pipeline_data())
                        } else {
                            Err(ShellError::GenericError(
                                "Cannot view value".to_string(),
                                "the command does not have a viewable block".to_string(),
                                Some(arg_span),
                                None,
                                Vec::new(),
                            ))
                        }
                    } else {
                        Err(ShellError::GenericError(
                            "Cannot view value".to_string(),
                            "the command does not have a viewable block".to_string(),
                            Some(arg_span),
                            None,
                            Vec::new(),
                        ))
                    }
                } else if let Some(module_id) = engine_state.find_module(val.as_bytes(), &[]) {
                    // arg is a module
                    let module = engine_state.get_module(module_id);
                    if let Some(module_span) = module.span {
                        let contents = engine_state.get_span_contents(&module_span);
                        Ok(Value::string(String::from_utf8_lossy(contents), call.head)
                            .into_pipeline_data())
                    } else {
                        Err(ShellError::GenericError(
                            "Cannot view value".to_string(),
                            "the module does not have a viewable block".to_string(),
                            Some(arg_span),
                            None,
                            Vec::new(),
                        ))
                    }
                } else if let Some(alias_id) = engine_state.find_alias(val.as_bytes(), &[]) {
                    let contents = &mut engine_state.get_alias(alias_id).iter().map(|span| {
                        String::from_utf8_lossy(engine_state.get_span_contents(span)).to_string()
                    });
                    Ok(Value::String {
                        val: contents.join(" "),
                        span: call.head,
                    }
                    .into_pipeline_data())
                } else {
                    Err(ShellError::GenericError(
                        "Cannot view value".to_string(),
                        "this name does not correspond to a viewable value".to_string(),
                        Some(arg_span),
                        None,
                        Vec::new(),
                    ))
                }
            }
            _ => Err(ShellError::GenericError(
                "Cannot view value".to_string(),
                "this value cannot be viewed".to_string(),
                Some(arg_span),
                None,
                Vec::new(),
            )),
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "View the source of a code block",
                example: r#"let abc = { echo 'hi' }; view-source $abc"#,
                result: Some(Value::String {
                    val: "{ echo 'hi' }".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "View the source of a custom command",
                example: r#"def hi [] { echo 'Hi!' }; view-source hi"#,
                result: Some(Value::String {
                    val: "{ echo 'Hi!' }".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "View the source of a custom command, which participates in the caller environment",
                example: r#"def-env foo [] { let-env BAR = 'BAZ' }; view-source foo"#,
                result: Some(Value::String {
                    val: "{ let-env BAR = 'BAZ' }".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "View the source of a module",
                example: r#"module mod-foo { export-env { let-env FOO_ENV = 'BAZ' } }; view-source mod-foo"#,
                result: Some(Value::String {
                    val: " export-env { let-env FOO_ENV = 'BAZ' }".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "View the source of an alias",
                example: r#"alias hello = echo hi; view-source hello"#,
                result: Some(Value::String {
                    val: "echo hi".to_string(),
                    span: Span::test_data(),
                }),
            },
        ]
    }
}
