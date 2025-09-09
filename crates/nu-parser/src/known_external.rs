use nu_engine::command_prelude::*;
use nu_protocol::{
    CustomExample,
    ast::{self, Expr, Expression},
    engine::{self, CallImpl, CommandType, UNKNOWN_SPAN_ID},
    ir::{self, DataSlice},
};

#[derive(Clone)]
pub struct KnownExternal {
    pub signature: Box<Signature>,
    pub attributes: Vec<(String, Value)>,
    pub examples: Vec<CustomExample>,
}

impl Command for KnownExternal {
    fn name(&self) -> &str {
        &self.signature.name
    }

    fn signature(&self) -> Signature {
        *self.signature.clone()
    }

    fn description(&self) -> &str {
        &self.signature.description
    }

    fn extra_description(&self) -> &str {
        &self.signature.extra_description
    }

    fn search_terms(&self) -> Vec<&str> {
        self.signature
            .search_terms
            .iter()
            .map(String::as_str)
            .collect()
    }

    fn command_type(&self) -> CommandType {
        CommandType::External
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head_span = call.head;
        let decl_id = engine_state
            .find_decl("run-external".as_bytes(), &[])
            .ok_or(ShellError::ExternalNotSupported { span: head_span })?;

        let command = engine_state.get_decl(decl_id);

        let extern_name = if let Some(name_bytes) = engine_state.find_decl_name(call.decl_id, &[]) {
            String::from_utf8_lossy(name_bytes)
        } else {
            return Err(ShellError::NushellFailedSpanned {
                msg: "known external name not found".to_string(),
                label: "could not find name for this command".to_string(),
                span: call.head,
            });
        };

        let extern_name: Vec<_> = extern_name.split(' ').collect();

        match &call.inner {
            CallImpl::AstRef(call) => {
                let extern_call = ast_call_to_extern_call(engine_state, call, &extern_name)?;
                command.run(engine_state, stack, &(&extern_call).into(), input)
            }
            CallImpl::AstBox(call) => {
                let extern_call = ast_call_to_extern_call(engine_state, call, &extern_name)?;
                command.run(engine_state, stack, &(&extern_call).into(), input)
            }
            CallImpl::IrRef(call) => {
                let extern_call = ir_call_to_extern_call(stack, call, &extern_name)?;
                command.run(engine_state, stack, &(&extern_call).into(), input)
            }
            CallImpl::IrBox(call) => {
                let extern_call = ir_call_to_extern_call(stack, call, &extern_name)?;
                command.run(engine_state, stack, &(&extern_call).into(), input)
            }
        }
    }

    fn attributes(&self) -> Vec<(String, Value)> {
        self.attributes.clone()
    }

    fn examples(&self) -> Vec<Example<'_>> {
        self.examples
            .iter()
            .map(CustomExample::to_example)
            .collect()
    }
}

/// Transform the args from an `ast::Call` onto a `run-external` call
fn ast_call_to_extern_call(
    engine_state: &EngineState,
    call: &ast::Call,
    extern_name: &[&str],
) -> Result<ast::Call, ShellError> {
    let head_span = call.head;

    let mut extern_call = ast::Call::new(head_span);

    let call_head_id = engine_state
        .find_span_id(call.head)
        .unwrap_or(UNKNOWN_SPAN_ID);

    let arg_extern_name = Expression::new_existing(
        Expr::String(extern_name[0].to_string()),
        call.head,
        call_head_id,
        Type::String,
    );

    extern_call.add_positional(arg_extern_name);

    for subcommand in extern_name.iter().skip(1) {
        extern_call.add_positional(Expression::new_existing(
            Expr::String(subcommand.to_string()),
            call.head,
            call_head_id,
            Type::String,
        ));
    }

    for arg in &call.arguments {
        match arg {
            ast::Argument::Positional(positional) => extern_call.add_positional(positional.clone()),
            ast::Argument::Named(named) => {
                let named_span_id = engine_state
                    .find_span_id(named.0.span)
                    .unwrap_or(UNKNOWN_SPAN_ID);
                if let Some(short) = &named.1 {
                    extern_call.add_positional(Expression::new_existing(
                        Expr::String(format!("-{}", short.item)),
                        named.0.span,
                        named_span_id,
                        Type::String,
                    ));
                } else {
                    extern_call.add_positional(Expression::new_existing(
                        Expr::String(format!("--{}", named.0.item)),
                        named.0.span,
                        named_span_id,
                        Type::String,
                    ));
                }
                if let Some(arg) = &named.2 {
                    extern_call.add_positional(arg.clone());
                }
            }
            ast::Argument::Unknown(unknown) => extern_call.add_unknown(unknown.clone()),
            ast::Argument::Spread(args) => extern_call.add_spread(args.clone()),
        }
    }

    Ok(extern_call)
}

/// Transform the args from an `ir::Call` onto a `run-external` call
fn ir_call_to_extern_call(
    stack: &mut Stack,
    call: &ir::Call,
    extern_name: &[&str],
) -> Result<ir::Call, ShellError> {
    let mut extern_call = ir::Call::build(call.decl_id, call.head);

    // Add the command and subcommands
    for name in extern_name {
        extern_call.add_positional(stack, call.head, Value::string(*name, call.head));
    }

    // Add the arguments, reformatting named arguments into string positionals
    for index in 0..call.args_len {
        match &call.arguments(stack)[index] {
            engine::Argument::Flag {
                data,
                name,
                short,
                span,
            } => {
                let name_arg = engine::Argument::Positional {
                    span: *span,
                    val: Value::string(known_external_option_name(data, *name, *short), *span),
                    ast: None,
                };
                extern_call.add_argument(stack, name_arg);
            }
            engine::Argument::Named {
                data,
                name,
                short,
                span,
                val,
                ..
            } => {
                let name_arg = engine::Argument::Positional {
                    span: *span,
                    val: Value::string(known_external_option_name(data, *name, *short), *span),
                    ast: None,
                };
                let val_arg = engine::Argument::Positional {
                    span: *span,
                    val: val.clone(),
                    ast: None,
                };
                extern_call.add_argument(stack, name_arg);
                extern_call.add_argument(stack, val_arg);
            }
            a @ (engine::Argument::Positional { .. }
            | engine::Argument::Spread { .. }
            | engine::Argument::ParserInfo { .. }) => {
                let argument = a.clone();
                extern_call.add_argument(stack, argument);
            }
        }
    }

    Ok(extern_call.finish())
}

fn known_external_option_name(data: &[u8], name: DataSlice, short: DataSlice) -> String {
    if !data[name].is_empty() {
        format!(
            "--{}",
            std::str::from_utf8(&data[name]).expect("invalid utf-8 in flag name")
        )
    } else {
        format!(
            "-{}",
            std::str::from_utf8(&data[short]).expect("invalid utf-8 in flag short name")
        )
    }
}
