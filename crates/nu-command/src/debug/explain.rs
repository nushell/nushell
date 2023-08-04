use nu_engine::{eval_expression, CallExt};
use nu_protocol::ast::{Argument, Block, Call, Expr, Expression};
use nu_protocol::engine::{Closure, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Span,
    SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Explain;

impl Command for Explain {
    fn name(&self) -> &str {
        "explain"
    }

    fn usage(&self) -> &str {
        "Explain closure contents."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("explain")
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "the closure to run",
            )
            .input_output_types(vec![(Type::Any, Type::Any), (Type::Nothing, Type::Any)])
            .allow_variants_without_examples(true)
            .category(Category::Debug)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        // This was all delightfully stolen from benchmark :)
        let capture_block: Closure = call.req(engine_state, stack, 0)?;
        let block = engine_state.get_block(capture_block.block_id);
        let ctrlc = engine_state.ctrlc.clone();
        let mut stack = stack.captures_to_stack(&capture_block.captures);

        let elements = get_pipeline_elements(engine_state, &mut stack, block)?;

        Ok(elements.into_pipeline_data(ctrlc))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Explain a command within a closure",
            example: "explain {|| ls | sort-by name type -i | get name } | table -e",
            result: None,
        }]
    }
}

pub fn get_pipeline_elements(
    engine_state: &EngineState,
    stack: &mut Stack,
    block: &Block,
) -> Result<Vec<Value>, ShellError> {
    let mut element_values = vec![];
    let span = Span::test_data();

    for (pipeline_idx, pipeline) in block.pipelines.iter().enumerate() {
        let mut i = 0;
        while i < pipeline.elements.len() {
            let pipeline_element = &pipeline.elements[i];
            let pipeline_expression = pipeline_element.expression().clone();
            let pipeline_span = pipeline_element.span();
            let element_str =
                String::from_utf8_lossy(engine_state.get_span_contents(pipeline_span));
            let value = Value::string(element_str.to_string(), pipeline_span);
            let expr = pipeline_expression.expr.clone();
            let (command_name, command_args_value) = if let Expr::Call(call) = expr {
                let command = engine_state.get_decl(call.decl_id);
                (
                    command.name().to_string(),
                    get_arguments(engine_state, stack, *call),
                )
            } else {
                ("no-op".to_string(), vec![])
            };
            let index = format!("{pipeline_idx}_{i}");
            let value_type = value.get_type();
            let value_span = value.span()?;
            let value_span_start = value_span.start as i64;
            let value_span_end = value_span.end as i64;
            let command_name = command_name;

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
            i += 1;
        }
    }
    Ok(element_values)
}

fn get_arguments(engine_state: &EngineState, stack: &mut Stack, call: Call) -> Vec<Value> {
    let mut arg_value = vec![];
    let span = Span::test_data();
    for arg in &call.arguments {
        match arg {
            // I think the second argument to Argument::Named is the short name, but I'm not really sure.
            // Please fix it if it's wrong. :)
            Argument::Named((name, short, opt_expr)) => {
                let arg_type = "named";
                let arg_value_name = name.item.clone();
                let arg_value_name_span_start = name.span.start as i64;
                let arg_value_name_span_end = name.span.end as i64;

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

                if let Some(shortcut) = short {
                    let arg_type = "short";
                    let arg_value_name = shortcut.item.clone();
                    let arg_value_name_span_start = shortcut.span.start as i64;
                    let arg_value_name_span_end = shortcut.span.end as i64;

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
                            Value::string(arg_value_name, shortcut.span),
                            Value::string("string".to_string(), span),
                            Value::int(arg_value_name_span_start, span),
                            Value::int(arg_value_name_span_end, span),
                        ],
                        span: name.span,
                    };
                    arg_value.push(rec);
                };

                if let Some(expression) = opt_expr {
                    let evaluated_expression =
                        get_expression_as_value(engine_state, stack, expression);
                    let arg_type = "expr";
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
                            Value::string(arg_value_name, expression.span),
                            Value::string(arg_value_type, span),
                            Value::int(arg_value_name_span_start, span),
                            Value::int(arg_value_name_span_end, span),
                        ],
                        span: expression.span,
                    };
                    arg_value.push(rec);
                };
            }
            Argument::Positional(inner_expr) => {
                let arg_type = "positional";
                let evaluated_expression = get_expression_as_value(engine_state, stack, inner_expr);
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
            Argument::Unknown(inner_expr) => {
                let arg_type = "unknown";
                let evaluated_expression = get_expression_as_value(engine_state, stack, inner_expr);
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
        };
    }

    arg_value
}

fn get_expression_as_value(
    engine_state: &EngineState,
    stack: &mut Stack,
    inner_expr: &Expression,
) -> Value {
    match eval_expression(engine_state, stack, inner_expr) {
        Ok(v) => v,
        Err(error) => Value::Error {
            error: Box::new(error),
        },
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
                .map(debug_string_without_formatting)
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
        //TODO: It would be good to drill in deeper to blocks and closures.
        Value::Block { val, .. } => format!("<Block {val}>"),
        Value::Closure { val, .. } => format!("<Closure {val}>"),
        Value::Null { .. } => String::new(),
        Value::Error { error } => format!("{error:?}"),
        Value::Binary { val, .. } => format!("{val:?}"),
        Value::CellPath { val, .. } => val.into_string(),
        Value::CustomValue { val, .. } => val.value_string(),
        Value::MatchPattern { val, .. } => format!("{:?}", val),
    }
}
