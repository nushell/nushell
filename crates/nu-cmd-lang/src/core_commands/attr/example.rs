use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct AttrExample;

impl Command for AttrExample {
    fn name(&self) -> &str {
        "attr example"
    }

    // TODO: When const closure are available, switch to using them for the `example` argument
    // rather than a block. That should remove the need for `requires_ast_for_arguments` to be true
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

        let example_string: Result<String, _> = call.req(engine_state, stack, 1);
        let example_expr = call
            .positional_nth(stack, 1)
            .ok_or(ShellError::MissingParameter {
                param_name: "example".into(),
                span: call.head,
            })?;

        let working_set = StateWorkingSet::new(engine_state);

        attr_example_impl(
            example_expr,
            example_string,
            &working_set,
            call,
            description,
            result,
        )
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let description: Spanned<String> = call.req_const(working_set, 0)?;
        let result: Option<Value> = call.get_flag_const(working_set, "result")?;

        let example_string: Result<String, _> = call.req_const(working_set, 1);
        let example_expr =
            call.assert_ast_call()?
                .positional_nth(1)
                .ok_or(ShellError::MissingParameter {
                    param_name: "example".into(),
                    span: call.head,
                })?;

        attr_example_impl(
            example_expr,
            example_string,
            working_set,
            call,
            description,
            result,
        )
    }

    fn is_const(&self) -> bool {
        true
    }

    fn requires_ast_for_arguments(&self) -> bool {
        true
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Add examples to custom command",
            example: r###"# Double numbers
    @example "double an int" { 2 | double } --result 4
    @example "double a float" { 0.25 | double } --result 0.5
    def double []: [number -> number] { $in * 2 }"###,
            result: None,
        }]
    }
}

fn attr_example_impl(
    example_expr: &nu_protocol::ast::Expression,
    example_string: Result<String, ShellError>,
    working_set: &StateWorkingSet<'_>,
    call: &Call<'_>,
    description: Spanned<String>,
    result: Option<Value>,
) -> Result<PipelineData, ShellError> {
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
        None => example_string?,
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
