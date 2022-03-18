use nu_protocol::ast::Expr;
use nu_protocol::engine::{EngineState, Stack, StateWorkingSet};
use nu_protocol::{ast::Call, engine::Command, ShellError, Signature};
use nu_protocol::{PipelineData, Spanned};

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
        // FIXME: This is a bit of a hack, and it'd be nice for the parser/AST to be able to handle the original
        // order of the parameters. Until then, we need to recover the original order.

        // FIXME: This is going to be a bit expensive, but we need to do it to ensure any new block/subexpression
        // we find when parsing the external call is handled properly.
        let mut engine_state = engine_state.clone();

        let call_span = call.span();
        let contents = engine_state.get_span_contents(&call_span);

        let redirect_stdout = call.redirect_stdout;
        let redirect_stderr = call.redirect_stderr;

        let (lexed, _) = crate::lex(contents, call_span.start, &[], &[], true);

        let spans: Vec<_> = lexed.into_iter().map(|x| x.span).collect();
        let mut working_set = StateWorkingSet::new(&engine_state);
        let (external_call, _) = crate::parse_external_call(&mut working_set, &spans, &[]);
        let delta = working_set.render();
        engine_state.merge_delta(delta, None, ".")?;

        match external_call.expr {
            Expr::ExternalCall(head, args) => {
                let decl_id = engine_state
                    .find_decl("run-external".as_bytes())
                    .ok_or(ShellError::ExternalNotSupported(head.span))?;

                let command = engine_state.get_decl(decl_id);

                let mut call = Call::new(head.span);

                call.positional.push(*head);

                for arg in args {
                    call.positional.push(arg.clone())
                }

                if redirect_stdout {
                    call.named.push((
                        Spanned {
                            item: "redirect-stdout".into(),
                            span: call_span,
                        },
                        None,
                    ))
                }

                if redirect_stderr {
                    call.named.push((
                        Spanned {
                            item: "redirect-stderr".into(),
                            span: call_span,
                        },
                        None,
                    ))
                }

                command.run(&engine_state, stack, &call, input)
            }
            x => {
                println!("{:?}", x);
                panic!("internal error: known external not actually external")
            }
        }
    }
}
