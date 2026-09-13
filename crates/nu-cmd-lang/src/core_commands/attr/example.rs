use nu_engine::command_prelude::*;
use nu_protocol::engine::Closure;

#[derive(Clone)]
pub struct AttrExample;

impl Command for AttrExample {
    fn name(&self) -> &str {
        "attr example"
    }

    // Example blocks are accepted so their source text can be extracted for help output.
    // Const eval materializes blocks as capture-free closures (see eval_const bridge).
    fn signature(&self) -> Signature {
        Signature::build("attr example")
            .input_output_types(vec![(
                Type::Nothing,
                Type::Record(
                    vec![
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
        let example_arg: Value = call.req(engine_state, stack, 1)?;

        let working_set = StateWorkingSet::new(engine_state);
        let (example_content, example_span) =
            example_content_from_value(&working_set, &example_arg)?;

        attr_example_record(
            call.head,
            description,
            example_content,
            example_span,
            result,
        )
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let description: Spanned<String> = call.req_const(working_set, stack, 0)?;
        let result: Option<Value> = call.get_flag_const(working_set, stack, "result")?;
        let example_arg: Value = call.req_const(working_set, stack, 1)?;

        let (example_content, example_span) =
            example_content_from_value(working_set, &example_arg)?;

        attr_example_record(
            call.head,
            description,
            example_content,
            example_span,
            result,
        )
    }

    fn is_const(&self) -> bool {
        true
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Add examples to custom command.",
            example: r###"# Double numbers
    @example "double an int" { 2 | double } --result 4
    @example "double a float" { 0.25 | double } --result 0.5
    def double []: [number -> number] { $in * 2 }"###,
            result: None,
        }]
    }
}

fn example_content_from_value(
    working_set: &StateWorkingSet<'_>,
    example_arg: &Value,
) -> Result<(String, Span), ShellError> {
    match example_arg {
        Value::String {
            val, internal_span, ..
        } => Ok((val.clone(), *internal_span)),
        Value::Closure {
            val, internal_span, ..
        } => {
            let content = block_source_from_closure(working_set, val, *internal_span)?;
            Ok((content, *internal_span))
        }
        other => Err(ShellError::CantConvert {
            to_type: "string or block".into(),
            from_type: other.get_type().to_string(),
            span: other.span(),
            help: None,
        }),
    }
}

fn block_source_from_closure(
    working_set: &StateWorkingSet<'_>,
    closure: &Closure,
    span: Span,
) -> Result<String, ShellError> {
    let block = working_set.get_block(closure.block_id);
    let block_span = block.span.ok_or_else(|| ShellError::CantConvert {
        to_type: "string".into(),
        from_type: "closure".into(),
        span,
        help: Some(format!(
            "unable to retrieve block contents for closure with id {}",
            closure.block_id.get()
        )),
    })?;
    Ok(block_source_string(working_set, block_span))
}

fn block_source_string(working_set: &StateWorkingSet<'_>, span: Span) -> String {
    let contents = working_set.get_span_contents(span);
    let contents = contents
        .strip_prefix(b"{")
        .and_then(|x| x.strip_suffix(b"}"))
        .unwrap_or(contents)
        .trim_ascii();
    String::from_utf8_lossy(contents).into_owned()
}

fn attr_example_record(
    head: Span,
    description: Spanned<String>,
    example_content: String,
    example_span: Span,
    result: Option<Value>,
) -> Result<PipelineData, ShellError> {
    let mut rec = record! {
        "description" => Value::string(description.item, description.span),
        "example" => Value::string(example_content, example_span),
    };
    if let Some(result) = result {
        rec.push("result", result);
    }

    Ok(Value::record(rec, head).into_pipeline_data())
}
