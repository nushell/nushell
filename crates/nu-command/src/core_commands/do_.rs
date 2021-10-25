use nu_engine::{eval_block, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{PipelineData, Signature, SyntaxShape, Value};

#[derive(Clone)]
pub struct Do;

impl Command for Do {
    fn name(&self) -> &str {
        "do"
    }

    fn usage(&self) -> &str {
        "Run a block"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("do")
            .desc(self.usage())
            .required(
                "block",
                SyntaxShape::Block(Some(vec![])),
                "the block to run",
            )
            .rest("rest", SyntaxShape::Any, "the parameter(s) for the block")
    }

    fn run(
        &self,
        context: &EvaluationContext,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let block_id = call.positional[0]
            .as_block()
            .expect("internal error: expected block");
        let rest: Vec<Value> = call.rest(context, 1)?;

        let block = context.engine_state.get_block(block_id);

        let state = context.enter_scope();

        let params: Vec<_> = block
            .signature
            .required_positional
            .iter()
            .chain(block.signature.optional_positional.iter())
            .collect();

        for param in params.iter().zip(&rest) {
            if let Some(var_id) = param.0.var_id {
                state.add_var(var_id, param.1.clone())
            }
        }

        if let Some(param) = &block.signature.rest_positional {
            if rest.len() > params.len() {
                let mut rest_items = vec![];

                for r in rest.into_iter().skip(params.len()) {
                    rest_items.push(r);
                }

                let span = if let Some(rest_item) = rest_items.first() {
                    rest_item.span()?
                } else {
                    call.head
                };

                state.add_var(
                    param
                        .var_id
                        .expect("Internal error: rest positional parameter lacks var_id"),
                    Value::List {
                        vals: rest_items,
                        span,
                    },
                )
            }
        }
        eval_block(&state, block, input)
    }
}
