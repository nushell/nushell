use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct AttrExample;

impl Command for AttrExample {
    fn name(&self) -> &str {
        "attr example"
    }

    fn signature(&self) -> Signature {
        Signature::build("attr example")
            .input_output_types(vec![(
                Type::Nothing,
                Type::Record(
                    [
                        ("description".into(), Type::String),
                        ("example".into(), Type::String),
                    ]
                    .into(),
                ),
            )])
            .allow_variants_without_examples(true)
            .required(
                "description",
                SyntaxShape::String,
                "Description of the example.",
            )
            .required(
                "example",
                SyntaxShape::OneOf(vec![SyntaxShape::Block, SyntaxShape::String]),
                "Example code snippet.",
            )
            .named(
                "result",
                SyntaxShape::Any,
                "Expected output of example.",
                None,
            )
            .category(Category::Core)
    }

    fn description(&self) -> &str {
        "Attribute for adding examples to custom commands."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let description: Spanned<String> = call.req(engine_state, stack, 0)?;
        let result: Option<Value> = call.get_flag(engine_state, stack, "result")?;
        let example = call
            .positional_nth(stack, 1)
            .ok_or(ShellError::MissingParameter {
                param_name: "example".into(),
                span: call.head,
            })?;

        let example_content = match example.as_block() {
            Some(block_id) => {
                let block = engine_state.get_block(block_id);
                let contents =
                    engine_state.get_span_contents(block.span.expect("a block must have a span"));
                let contents = contents
                    .strip_prefix(b"{")
                    .and_then(|x| x.strip_suffix(b"}"))
                    .unwrap_or(contents)
                    .trim_ascii();
                String::from_utf8_lossy(contents).into_owned()
            }
            None => match example.as_string() {
                Some(v) => v,
                None => panic!("internal error: missing block"),
            },
        };

        let mut rec = record! {
            "description" => Value::string(description.item, description.span),
            "example" => Value::string(example_content, example.span),
        };
        if let Some(result) = result {
            rec.push("result", result);
        }

        Ok(Value::record(rec, call.head).into_pipeline_data())
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let description: Spanned<String> = call.req_const(working_set, 0)?;
        let result: Option<Value> = call.get_flag_const(working_set, "result")?;
        let example_expr =
            call.assert_ast_call()?
                .positional_nth(1)
                .ok_or(ShellError::MissingParameter {
                    param_name: "example".into(),
                    span: call.head,
                })?;

        let example_content = match example_expr.as_block() {
            Some(block_id) => {
                let block = working_set.get_block(block_id);
                let contents =
                    working_set.get_span_contents(block.span.expect("a block must have a span"));
                let contents = contents
                    .strip_prefix(b"{")
                    .and_then(|x| x.strip_suffix(b"}"))
                    .unwrap_or(contents)
                    .trim_ascii();
                String::from_utf8_lossy(contents).into_owned()
            }
            None => {
                let example: String = call.req_const(working_set, 1)?;
                example
            }
        };

        let mut rec = record! {
            "description" => Value::string(description.item, description.span),
            "example" => Value::string(example_content, example_expr.span),
        };
        if let Some(result) = result {
            rec.push("result", result);
        }

        Ok(Value::record(rec, call.head).into_pipeline_data())
    }

    fn is_const(&self) -> bool {
        true
    }

    fn requires_ast_for_arguments(&self) -> bool {
        true
    }
}
