use nu_protocol::engine::{EngineState, Stack, StateWorkingSet};
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
            .find_decl("run-external".as_bytes())
            .ok_or(ShellError::ExternalNotSupported(head_span))?;

        let command = engine_state.get_decl(decl_id);

        let mut extern_call = Call::new(head_span);

        let working_state = StateWorkingSet::new(engine_state);
        let extern_name = working_state.get_span_contents(call.head);
        let extern_name = String::from_utf8(extern_name.to_vec())
            .expect("this was already parsed as a command name");
        let arg_extern_name = Expression {
            expr: Expr::String(extern_name),
            span: call.head,
            ty: Type::String,
            custom_completion: None,
        };

        extern_call.add_positional(arg_extern_name);

        for arg in &call.arguments {
            match arg {
                Argument::Positional(positional) => extern_call.add_positional(positional.clone()),
                Argument::Named(named) => extern_call.add_named(named.clone()),
            }
        }

        if call.redirect_stdout {
            extern_call.add_named((
                Spanned {
                    item: "redirect-stdout".into(),
                    span: call_span,
                },
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
            ))
        }

        command.run(engine_state, stack, &extern_call, input)
    }
}
