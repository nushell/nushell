use nu_engine::{eval_block, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{CaptureBlock, Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, Signature, SyntaxShape, Value};

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
            .required("block", SyntaxShape::Any, "the block to run")
            .switch(
                "ignore-errors",
                "ignore errors as the block runs",
                Some('i'),
            )
            .rest("rest", SyntaxShape::Any, "the parameter(s) for the block")
            .category(Category::Core)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let block: CaptureBlock = call.req(engine_state, stack, 0)?;
        let rest: Vec<Value> = call.rest(engine_state, stack, 1)?;
        let ignore_errors = call.has_flag("ignore-errors");

        let mut stack = stack.captures_to_stack(&block.captures);
        let block = engine_state.get_block(block.block_id);

        let params: Vec<_> = block
            .signature
            .required_positional
            .iter()
            .chain(block.signature.optional_positional.iter())
            .collect();

        for param in params.iter().zip(&rest) {
            if let Some(var_id) = param.0.var_id {
                stack.add_var(var_id, param.1.clone())
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

                stack.add_var(
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
        let result = eval_block(
            engine_state,
            &mut stack,
            block,
            input,
            call.redirect_stdout,
            ignore_errors,
        );

        if ignore_errors {
            match result {
                Ok(x) => Ok(x),
                Err(_) => Ok(PipelineData::new(call.head)),
            }
        } else {
            result
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Run the block",
                example: r#"do { echo hello }"#,
                result: Some(Value::test_string("hello")),
            },
            Example {
                description: "Run the block and ignore errors",
                example: r#"do -i { thisisnotarealcommand }"#,
                result: None,
            },
            Example {
                description: "Run the block, with a positional parameter",
                example: r#"do {|x| 100 + $x } 50"#,
                result: Some(Value::test_int(150)),
            },
        ]
    }
}

mod test {
    #[test]
    fn test_examples() {
        use super::Do;
        use crate::test_examples;
        test_examples(Do {})
    }
}
