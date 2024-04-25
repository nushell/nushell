use nu_engine::command_prelude::*;
use nu_protocol::ast::{Argument, Expr, Expression};

#[derive(Clone)]
pub struct KnownExternal {
    pub name: String,
    pub signature: Box<Signature>,
    pub usage: String,
    pub extra_usage: String,
}

impl Command for KnownExternal {
    fn name(&self) -> &str {
        &self.name
    }

    fn signature(&self) -> Signature {
        *self.signature.clone()
    }

    fn usage(&self) -> &str {
        &self.usage
    }

    fn is_known_external(&self) -> bool {
        true
    }

    fn is_builtin(&self) -> bool {
        false
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

        let mut extern_call = Call::new(head_span);

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

        let arg_extern_name = Expression {
            expr: Expr::String(extern_name[0].to_string()),
            span: call.head,
            ty: Type::String,
            custom_completion: None,
        };

        extern_call.add_positional(arg_extern_name);

        for subcommand in extern_name.into_iter().skip(1) {
            extern_call.add_positional(Expression {
                expr: Expr::String(subcommand.to_string()),
                span: call.head,
                ty: Type::String,
                custom_completion: None,
            });
        }

        for arg in &call.arguments {
            match arg {
                Argument::Positional(positional) => extern_call.add_positional(positional.clone()),
                Argument::Named(named) => {
                    if let Some(short) = &named.1 {
                        extern_call.add_positional(Expression {
                            expr: Expr::String(format!("-{}", short.item)),
                            span: named.0.span,
                            ty: Type::String,
                            custom_completion: None,
                        });
                    } else {
                        extern_call.add_positional(Expression {
                            expr: Expr::String(format!("--{}", named.0.item)),
                            span: named.0.span,
                            ty: Type::String,
                            custom_completion: None,
                        });
                    }
                    if let Some(arg) = &named.2 {
                        extern_call.add_positional(arg.clone());
                    }
                }
                Argument::Unknown(unknown) => extern_call.add_unknown(unknown.clone()),
                Argument::Spread(args) => extern_call.add_spread(args.clone()),
            }
        }

        command.run(engine_state, stack, &extern_call, input)
    }
}
