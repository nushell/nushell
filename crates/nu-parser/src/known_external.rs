use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::{
    ast::{Argument, Call, Expr, Expression},
    engine::Command,
    ShellError, Signature,
};
use nu_protocol::{PipelineData, Spanned, Type};

#[derive(Clone)]
pub struct KnownExternal {
    pub name: String,
    pub signature: Box<Signature>,
    pub usage: String,
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
        let call_span = call.span();
        let head_span = call.head;
        let decl_id = engine_state
            .find_decl("run-external".as_bytes(), &[])
            .ok_or(ShellError::ExternalNotSupported(head_span))?;

        let command = engine_state.get_decl(decl_id);

        let mut extern_call = Call::new(head_span);

        let extern_name = engine_state.get_decl(call.decl_id).name();

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
            }
        }

        if call.redirect_stdout {
            extern_call.add_named((
                Spanned {
                    item: "redirect-stdout".into(),
                    span: call_span,
                },
                None,
                None,
            ))
        }

        if call.redirect_stderr {
            extern_call.add_named((
                Spanned {
                    item: "redirect-stderr".into(),
                    span: call_span,
                },
                None,
                None,
            ))
        }

        command.run(engine_state, stack, &extern_call, input)
    }
}
