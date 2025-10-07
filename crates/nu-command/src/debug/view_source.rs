use nu_engine::command_prelude::*;
use nu_protocol::{Config, PipelineMetadata};

use std::fmt::Write;

#[derive(Clone)]
pub struct ViewSource;

impl Command for ViewSource {
    fn name(&self) -> &str {
        "view source"
    }

    fn description(&self) -> &str {
        "View a block, module, or a definition."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("view source")
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .required("item", SyntaxShape::Any, "Name or block to view.")
            .category(Category::Debug)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let arg: Value = call.req(engine_state, stack, 0)?;
        let arg_span = arg.span();

        let source = match arg {
            Value::Int { val, .. } => {
                if let Some(block) =
                    engine_state.try_get_block(nu_protocol::BlockId::new(val as usize))
                {
                    if let Some(span) = block.span {
                        let contents = engine_state.get_span_contents(span);
                        Ok(Value::string(String::from_utf8_lossy(contents), call.head)
                            .into_pipeline_data())
                    } else {
                        Err(ShellError::GenericError {
                            error: "Cannot view int value".to_string(),
                            msg: "the block does not have a viewable span".to_string(),
                            span: Some(arg_span),
                            help: None,
                            inner: vec![],
                        })
                    }
                } else {
                    Err(ShellError::GenericError {
                        error: format!("Block Id {} does not exist", arg.coerce_into_string()?),
                        msg: "this number does not correspond to a block".to_string(),
                        span: Some(arg_span),
                        help: None,
                        inner: vec![],
                    })
                }
            }

            Value::String { val, .. } => {
                if let Some(decl_id) = engine_state.find_decl(val.as_bytes(), &[]) {
                    // arg is a command
                    let decl = engine_state.get_decl(decl_id);
                    let sig = decl.signature();
                    let vec_of_required = &sig.required_positional;
                    let vec_of_optional = &sig.optional_positional;
                    let rest = &sig.rest_positional;
                    let vec_of_flags = &sig.named;
                    let type_signatures = &sig.input_output_types;

                    if decl.is_alias() {
                        if let Some(alias) = &decl.as_alias() {
                            let contents = String::from_utf8_lossy(
                                engine_state.get_span_contents(alias.wrapped_call.span),
                            );
                            Ok(Value::string(contents, call.head).into_pipeline_data())
                        } else {
                            Ok(Value::string("no alias found", call.head).into_pipeline_data())
                        }
                    }
                    // gets vector of positionals.
                    else if let Some(block_id) = decl.block_id() {
                        let block = engine_state.get_block(block_id);
                        if let Some(block_span) = block.span {
                            let contents = engine_state.get_span_contents(block_span);
                            // name of function
                            let mut final_contents = String::new();
                            if val.contains(' ') {
                                let _ = write!(&mut final_contents, "def \"{val}\" [");
                            } else {
                                let _ = write!(&mut final_contents, "def {val} [");
                            };
                            if !vec_of_required.is_empty()
                                || !vec_of_optional.is_empty()
                                || vec_of_flags.len() != 1
                                || rest.is_some()
                            {
                                final_contents.push(' ');
                            }
                            for n in vec_of_required {
                                let _ = write!(&mut final_contents, "{}: {} ", n.name, n.shape);
                                // positional arguments
                            }
                            for n in vec_of_optional {
                                if let Some(s) = n.default_value.clone() {
                                    let _ = write!(
                                        &mut final_contents,
                                        "{}: {} = {} ",
                                        n.name,
                                        n.shape,
                                        s.to_expanded_string(" ", &Config::default())
                                    );
                                } else {
                                    let _ =
                                        write!(&mut final_contents, "{}?: {} ", n.name, n.shape);
                                }
                            }
                            for n in vec_of_flags {
                                // skip adding the help flag
                                if n.long == "help" {
                                    continue;
                                }
                                let _ = write!(&mut final_contents, "--{}", n.long);
                                if let Some(short) = n.short {
                                    let _ = write!(&mut final_contents, "(-{short})");
                                }
                                if let Some(arg) = &n.arg {
                                    let _ = write!(&mut final_contents, ": {arg}");
                                }
                                final_contents.push(' ');
                            }
                            if let Some(rest_arg) = rest {
                                let _ = write!(
                                    &mut final_contents,
                                    "...{}:{}",
                                    rest_arg.name, rest_arg.shape
                                );
                            }
                            let len = type_signatures.len();
                            if len != 0 {
                                final_contents.push_str("]: [");
                                let mut c = 0;
                                for (insig, outsig) in type_signatures {
                                    c += 1;
                                    let s = format!("{insig} -> {outsig}");
                                    final_contents.push_str(&s);
                                    if c != len {
                                        final_contents.push_str(", ")
                                    }
                                }
                            }
                            final_contents.push_str("] ");
                            final_contents.push_str(&String::from_utf8_lossy(contents));
                            Ok(Value::string(final_contents, call.head).into_pipeline_data())
                        } else {
                            Err(ShellError::GenericError {
                                error: "Cannot view string value".to_string(),
                                msg: "the command does not have a viewable block span".to_string(),
                                span: Some(arg_span),
                                help: None,
                                inner: vec![],
                            })
                        }
                    } else {
                        Err(ShellError::GenericError {
                            error: "Cannot view string decl value".to_string(),
                            msg: "the command does not have a viewable block".to_string(),
                            span: Some(arg_span),
                            help: None,
                            inner: vec![],
                        })
                    }
                } else if let Some(module_id) = engine_state.find_module(val.as_bytes(), &[]) {
                    // arg is a module
                    let module = engine_state.get_module(module_id);
                    if let Some(module_span) = module.span {
                        let contents = engine_state.get_span_contents(module_span);
                        Ok(Value::string(String::from_utf8_lossy(contents), call.head)
                            .into_pipeline_data())
                    } else {
                        Err(ShellError::GenericError {
                            error: "Cannot view string module value".to_string(),
                            msg: "the module does not have a viewable block".to_string(),
                            span: Some(arg_span),
                            help: None,
                            inner: vec![],
                        })
                    }
                } else {
                    Err(ShellError::GenericError {
                        error: "Cannot view string value".to_string(),
                        msg: "this name does not correspond to a viewable value".to_string(),
                        span: Some(arg_span),
                        help: None,
                        inner: vec![],
                    })
                }
            }
            value => {
                if let Ok(closure) = value.as_closure() {
                    let block = engine_state.get_block(closure.block_id);

                    if let Some(span) = block.span {
                        let contents = engine_state.get_span_contents(span);
                        Ok(Value::string(String::from_utf8_lossy(contents), call.head)
                            .into_pipeline_data())
                    } else {
                        Ok(Value::string("<internal command>", call.head).into_pipeline_data())
                    }
                } else {
                    Err(ShellError::GenericError {
                        error: "Cannot view value".to_string(),
                        msg: "this value cannot be viewed".to_string(),
                        span: Some(arg_span),
                        help: None,
                        inner: vec![],
                    })
                }
            }
        };
        source.map(|x| {
            x.set_metadata(Some(PipelineMetadata {
                content_type: Some("application/x-nuscript".into()),
                ..Default::default()
            }))
        })
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "View the source of a code block",
                example: r#"let abc = {|| echo 'hi' }; view source $abc"#,
                result: Some(Value::test_string("{|| echo 'hi' }")),
            },
            Example {
                description: "View the source of a custom command",
                example: r#"def hi [] { echo 'Hi!' }; view source hi"#,
                result: Some(Value::test_string("def hi [] { echo 'Hi!' }")),
            },
            Example {
                description: "View the source of a custom command, which participates in the caller environment",
                example: r#"def --env foo [] { $env.BAR = 'BAZ' }; view source foo"#,
                result: Some(Value::test_string("def foo [] { $env.BAR = 'BAZ' }")),
            },
            Example {
                description: "View the source of a custom command with flags and arguments",
                example: r#"def test [a?:any --b:int ...rest:string] { echo 'test' }; view source test"#,
                result: Some(Value::test_string(
                    "def test [ a?: any --b: int ...rest: string] { echo 'test' }",
                )),
            },
            Example {
                description: "View the source of a module",
                example: r#"module mod-foo { export-env { $env.FOO_ENV = 'BAZ' } }; view source mod-foo"#,
                result: Some(Value::test_string(" export-env { $env.FOO_ENV = 'BAZ' }")),
            },
            Example {
                description: "View the source of an alias",
                example: r#"alias hello = echo hi; view source hello"#,
                result: Some(Value::test_string("echo hi")),
            },
        ]
    }
}
