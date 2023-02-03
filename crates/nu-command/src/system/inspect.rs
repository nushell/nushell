use nu_engine::{eval_expression, CallExt};
use nu_protocol::ast::{Argument, Block, Call, Expr, Expression};
use nu_protocol::engine::{Closure, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Span,
    SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Inspect;

impl Command for Inspect {
    fn name(&self) -> &str {
        "inspect"
    }

    fn usage(&self) -> &str {
        "Inspect the running closure"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("inspect")
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "the closure to run",
            )
            .input_output_types(vec![
                (Type::Any, Type::Duration),
                (Type::Nothing, Type::Duration),
            ])
            .allow_variants_without_examples(true)
            .category(Category::System)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let capture_block: Closure = call.req(engine_state, stack, 0)?;
        let block = engine_state.get_block(capture_block.block_id);
        let ctrlc = engine_state.ctrlc.clone();

        // let redirect_stdout = call.redirect_stdout;
        // let redirect_stderr = call.redirect_stderr;

        let mut stack = stack.captures_to_stack(&capture_block.captures);

        // In order to provide the pipeline as a positional, it must be converted into a value.
        // But because pipelines do not have Clone, this one has to be cloned as a value
        // and then converted back into a pipeline for eval_block().
        // So, the metadata must be saved here and restored at that point.
        // let input_metadata = input.metadata();
        let input_val = input.into_value(call.head);

        if let Some(var) = block.signature.get_positional(0) {
            if let Some(var_id) = &var.var_id {
                stack.add_var(*var_id, input_val.clone());
            }
        }

        let elements = get_pipeline_elements(engine_state, stack, &block)?;
        // eprintln!("Pipeline Elements: {:?}", elements);
        // for el in elements {
        //     eprintln!("{{ {el} }}");
        // }

        // Get the start time after all other computation has been done.
        // let start_time = Instant::now();
        // eval_block(
        //     engine_state,
        //     &mut stack,
        //     block,
        //     input_val.into_pipeline_data_with_metadata(input_metadata),
        //     redirect_stdout,
        //     redirect_stderr,
        // )?
        // .into_value(call.head);

        // let end_time = Instant::now();

        // let output = Value::Duration {
        //     val: (end_time - start_time).as_nanos() as i64,
        //     span: call.head,
        // };

        // Ok(output.into_pipeline_data())
        Ok(elements.into_pipeline_data(ctrlc))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Inspect a command within a closure",
                example: "inspect { sleep 500ms }",
                result: None,
            },
            Example {
                description: "Inspect a command using an existing input",
                example: "http get https://www.nushell.sh/book/ | inspect { split chars }",
                result: None,
            },
        ]
    }
}

pub fn get_pipeline_elements(
    engine_state: &EngineState,
    stack: Stack,
    block: &Block,
    // mut input: PipelineData,
    // redirect_stdout: bool,
    // redirect_stderr: bool,
) -> Result<Vec<Value>, ShellError> {
    // let mut elements = vec![];
    let mut element_values = vec![];
    let span = Span::test_data();
    // let mut commands = vec![];

    for (pipeline_idx, pipeline) in block.pipelines.iter().enumerate() {
        let mut i = 0;
        while i < pipeline.elements.len() {
            // let mut cols = vec![];
            // let mut vals = vec![];

            let pipeline_element = &pipeline.elements[i];
            let pipeline_expression = pipeline_element.expression().clone();
            let pipeline_span = &pipeline_element.span();
            let element_str =
                String::from_utf8_lossy(engine_state.get_span_contents(pipeline_span));
            let value = Value::string(element_str.to_string(), *pipeline_span);
            let expr = pipeline_expression.expr.clone();
            let (command_name, command_args_value) = if let Expr::Call(call) = expr {
                let command = engine_state.get_decl(call.decl_id);
                (
                    command.name().to_string(),
                    get_arguments(engine_state, stack.clone(), call),
                )
            } else {
                // ("no-op".to_string(), "no-args".to_string())
                // ("no-op".to_string(), (vec![], "no-args".to_string()))
                ("no-op".to_string(), vec![])
            };
            let index = format!("{pipeline_idx}_{i}");
            let value_type = value.get_type();
            let value_span = value.span()?;
            let value_span_start = value_span.start as i64;
            let value_span_end = value_span.end as i64;
            let command_name = command_name;
            // let element = format!("\"index\": \"{index}\", \"value_type\": \"{value_type}\", \"value_span_start\": {value_span_start}, \"value_span_end\": {value_span_end}, \"command_name\": \"{command_name}\", \"arguments\": {{ {command_args_str:?} }}");
            // elements.push(element);
            // cols.push("index".to_string());
            // vals.push(Value::string(index, span));
            // cols.push("value_type".to_string());
            // vals.push(Value::string(value_type.to_string(), span));
            // cols.push("value_span_start".to_string());
            // vals.push(Value::int(value_span_start, span));
            // cols.push("value_span_end".to_string());
            // vals.push(Value::int(value_span_end, span));
            // cols.push("command_name".to_string());
            // vals.push(Value::string(command_name, value_span));
            // cols.push("arguments".to_string());
            // vals.push(command_args);
            let rec = Value::Record {
                cols: vec![
                    "cmd_index".to_string(),
                    "cmd_name".to_string(),
                    "type".to_string(),
                    "cmd_args".to_string(),
                    "span_start".to_string(),
                    "span_end".to_string(),
                ],
                vals: vec![
                    Value::string(index, span),
                    Value::string(command_name, value_span),
                    Value::string(value_type.to_string(), span),
                    Value::List {
                        vals: command_args_value,
                        span: value_span,
                    },
                    Value::int(value_span_start, span),
                    Value::int(value_span_end, span),
                ],
                span: value_span,
            };
            element_values.push(rec);
            // commands.push(Value::Record { cols, vals, span });
            i += 1;
        }
    }
    // Ok(elements)
    Ok(element_values)
}

fn get_arguments(engine_state: &EngineState, stack: Stack, call: Box<Call>) -> Vec<Value> {
    let mut arg_value = vec![];
    // let mut idx = 0;
    let span = Span::test_data();
    for arg in &call.arguments {
        match arg {
            Argument::Named((name, something, opt_expr)) => {
                let arg_type = "named";
                let arg_value_name = name.item.clone();
                let arg_value_name_span_start = name.span.start as i64;
                let arg_value_name_span_end = name.span.end as i64;
                // arguments.push(format!(
                //     "\"arg_type{idx}\": \"{arg_type}\", \"arg_value_name{idx}\": \"{arg_value_name}\", \"arg_value_type{idx}\": \"string\", \"start{idx}\": {arg_value_name_span_start}, \"end{idx}\": {arg_value_name_span_end}"
                // ));

                let rec = Value::Record {
                    cols: vec![
                        "arg_type".to_string(),
                        "name".to_string(),
                        "type".to_string(),
                        "span_start".to_string(),
                        "span_end".to_string(),
                    ],
                    vals: vec![
                        Value::string(arg_type, span),
                        Value::string(arg_value_name, name.span),
                        Value::string("string".to_string(), span),
                        Value::int(arg_value_name_span_start, span),
                        Value::int(arg_value_name_span_end, span),
                    ],
                    span: name.span,
                };
                arg_value.push(rec);

                if let Some(thing) = something {
                    let arg_type = "thing";
                    let arg_value_name = thing.item.clone();
                    let arg_value_name_span_start = thing.span.start as i64;
                    let arg_value_name_span_end = thing.span.end as i64;
                    // arguments.push(format!(
                    //     "\"thing_type{idx}\": \"{arg_type}\", \"thing_name{idx}\": \"{arg_value_name}\", \"start{idx}\": {arg_value_name_span_start}, \"end{idx}\": {arg_value_name_span_end}"
                    // ));

                    let rec = Value::Record {
                        cols: vec![
                            "arg_type".to_string(),
                            "name".to_string(),
                            "type".to_string(),
                            "span_start".to_string(),
                            "span_end".to_string(),
                        ],
                        vals: vec![
                            Value::string(arg_type, span),
                            Value::string(arg_value_name, thing.span),
                            Value::string("string".to_string(), span),
                            Value::int(arg_value_name_span_start, span),
                            Value::int(arg_value_name_span_end, span),
                        ],
                        span: name.span,
                    };
                    arg_value.push(rec);
                } else {
                    // format!("\"thing_type\": \"thing\", \"thing_value\": \"None\"")
                    ()
                };
                // arguments.push(some_thing);

                if let Some(expression) = opt_expr {
                    let evaluated_expression =
                        get_expression_as_value(engine_state, stack.clone(), expression);
                    let arg_type = "expr";
                    let arg_value_name = debug_string_without_formatting(&evaluated_expression);
                    let arg_value_type = &evaluated_expression.get_type().to_string();
                    let evaled_span = evaluated_expression.expect_span();
                    let arg_value_name_span_start = evaled_span.start as i64;
                    let arg_value_name_span_end = evaled_span.end as i64;
                    // arguments.push(format!(
                    //     "\"arg_type\": \"{arg_type}\", \"arg_value_name{idx}\": \"{arg_value_name:?}\", \"arg_value_type{idx}\": \"{arg_value_type}\",  \"start{idx}\": {arg_value_name_span_start}, \"end{idx}\": {arg_value_name_span_end}"
                    // ));

                    let rec = Value::Record {
                        cols: vec![
                            "arg_type".to_string(),
                            "name".to_string(),
                            "type".to_string(),
                            "span_start".to_string(),
                            "span_end".to_string(),
                        ],
                        vals: vec![
                            Value::string(arg_type, span),
                            Value::string(arg_value_name, expression.span),
                            Value::string(arg_value_type, span),
                            Value::int(arg_value_name_span_start, span),
                            Value::int(arg_value_name_span_end, span),
                        ],
                        span: expression.span,
                    };
                    arg_value.push(rec);
                } else {
                    // format!("\"expr_type\": \"expr\", \"arg_value_name\": \"None\"")
                    // String::new()
                    ()
                };
                // arguments.push(some_expr);
            }
            Argument::Positional(inner_expr) => {
                let arg_type = "positional";
                let evaluated_expression =
                    get_expression_as_value(engine_state, stack.clone(), inner_expr);
                let arg_value_name = debug_string_without_formatting(&evaluated_expression);
                let arg_value_type = &evaluated_expression.get_type().to_string();
                let evaled_span = evaluated_expression.expect_span();
                let arg_value_name_span_start = evaled_span.start as i64;
                let arg_value_name_span_end = evaled_span.end as i64;
                // arguments.push(format!(
                //     "arg_type{idx}: {arg_type}, arg_value_name{idx}: {arg_value_name}, arg_value_type{idx}: {arg_value_type}, start{idx}: {arg_value_name_span_start}, end{idx}: {arg_value_name_span_end}"
                // ));

                let rec = Value::Record {
                    cols: vec![
                        "arg_type".to_string(),
                        "name".to_string(),
                        "type".to_string(),
                        "span_start".to_string(),
                        "span_end".to_string(),
                    ],
                    vals: vec![
                        Value::string(arg_type, span),
                        Value::string(arg_value_name, inner_expr.span),
                        Value::string(arg_value_type, span),
                        Value::int(arg_value_name_span_start, span),
                        Value::int(arg_value_name_span_end, span),
                    ],
                    span: inner_expr.span,
                };
                arg_value.push(rec);
            }
            Argument::Unknown(inner_expr) => {
                let arg_type = "unknown";
                let evaluated_expression =
                    get_expression_as_value(engine_state, stack.clone(), inner_expr);
                let arg_value_name = debug_string_without_formatting(&evaluated_expression);
                let arg_value_type = &evaluated_expression.get_type().to_string();
                let evaled_span = evaluated_expression.expect_span();
                let arg_value_name_span_start = evaled_span.start as i64;
                let arg_value_name_span_end = evaled_span.end as i64;

                let rec = Value::Record {
                    cols: vec![
                        "arg_type".to_string(),
                        "name".to_string(),
                        "type".to_string(),
                        "span_start".to_string(),
                        "span_end".to_string(),
                    ],
                    vals: vec![
                        Value::string(arg_type, span),
                        Value::string(arg_value_name, inner_expr.span),
                        Value::string(arg_value_type, span),
                        Value::int(arg_value_name_span_start, span),
                        Value::int(arg_value_name_span_end, span),
                    ],
                    span: inner_expr.span,
                };
                arg_value.push(rec);
            }

            // arguments.push(format!(
            //     "\"arg_type{idx}\": \"unknown\": \"{inner_expr:#?}\""
            // )),
        };
        // idx += 1;
    }

    arg_value
}

fn get_expression_as_value(
    engine_state: &EngineState,
    stack: Stack,
    inner_expr: &Expression,
) -> Value {
    match eval_expression(engine_state, &mut stack.clone(), inner_expr) {
        Ok(v) => v,
        Err(error) => Value::Error { error },
    }
}

pub fn debug_string_without_formatting(value: &Value) -> String {
    match value {
        Value::Bool { val, .. } => val.to_string(),
        Value::Int { val, .. } => val.to_string(),
        Value::Float { val, .. } => val.to_string(),
        Value::Filesize { val, .. } => val.to_string(),
        Value::Duration { val, .. } => val.to_string(),
        Value::Date { val, .. } => format!("{val:?}"),
        Value::Range { val, .. } => {
            format!(
                "{}..{}",
                debug_string_without_formatting(&val.from),
                debug_string_without_formatting(&val.to)
            )
        }
        Value::String { val, .. } => val.clone(),
        Value::List { vals: val, .. } => format!(
            "[{}]",
            val.iter()
                .map(|x| debug_string_without_formatting(x))
                .collect::<Vec<_>>()
                .join(" ")
        ),
        Value::Record { cols, vals, .. } => format!(
            "{{{}}}",
            cols.iter()
                .zip(vals.iter())
                .map(|(x, y)| format!("{}: {}", x, debug_string_without_formatting(y)))
                .collect::<Vec<_>>()
                .join(" ")
        ),
        Value::LazyRecord { val, .. } => match val.collect() {
            Ok(val) => debug_string_without_formatting(&val),
            Err(error) => format!("{error:?}"),
        },
        Value::Block { val, .. } => format!("<Block {val}>"),
        Value::Closure { val, .. } => format!("<Closure {val}>"),
        Value::Nothing { .. } => String::new(),
        Value::Error { error } => format!("{error:?}"),
        Value::Binary { val, .. } => format!("{val:?}"),
        Value::CellPath { val, .. } => val.into_string(),
        Value::CustomValue { val, .. } => val.value_string(),
    }
}
