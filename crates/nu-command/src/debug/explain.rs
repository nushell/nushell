use std::ops::ControlFlow;

use nu_engine::{command_prelude::*, get_eval_expression};
use nu_protocol::{
    ast::{self, Argument, Block, Expr, Expression, Traverse},
    engine::{Closure, StateWorkingSet},
};

#[derive(Clone)]
pub struct Explain;

impl Command for Explain {
    fn name(&self) -> &str {
        "explain"
    }

    fn description(&self) -> &str {
        "Explain closure contents."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("explain")
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "The closure to run.",
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
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        // This was all delightfully stolen from benchmark :)
        let capture_block: Closure = call.req(engine_state, stack, 0)?;
        let block = engine_state.get_closure_block(&capture_block, head)?;
        let mut stack = stack.captures_to_stack(capture_block.captures);
        let elements = get_pipeline_elements(engine_state, &mut stack, block, head);
        Ok(Value::list(elements, head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Explain a command within a closure.",
            example: "explain {|| ls | sort-by name type --ignore-case | get name } | table --expand",
            result: None,
        }]
    }
}

pub fn get_pipeline_elements(
    engine_state: &EngineState,
    stack: &mut Stack,
    block: &Block,
    span: Span,
) -> Vec<Value> {
    let eval_expression = get_eval_expression(engine_state);

    block
        .pipelines
        .iter()
        .enumerate()
        .flat_map(|(p_idx, pipeline)| {
            pipeline
                .elements
                .iter()
                .enumerate()
                .map(move |(e_idx, element)| (format!("{p_idx}_{e_idx}"), element))
        })
        .map(move |(cmd_index, element)| {
            let expression = &element.expr;
            let expr_span = element.expr.span;

            let (command_name, command_args_value, ty) = if let Expr::Call(call) = &expression.expr
            {
                let command = engine_state.get_decl(call.decl_id);
                (
                    command.name().to_string(),
                    get_arguments(engine_state, stack, call.as_ref(), eval_expression),
                    command
                        .signature()
                        .get_output_type(None)
                        .unwrap_or_default()
                        .to_string(),
                )
            } else {
                ("no-op".to_string(), vec![], expression.ty.to_string())
            };

            let record = record! {
                "cmd_index" => Value::string(cmd_index, span),
                "cmd_name" => Value::string(command_name, expr_span),
                "type" => Value::string(ty, span),
                "cmd_args" => Value::list(command_args_value, expr_span),
                "span_start" => Value::int(expr_span.start as i64, span),
                "span_end" => Value::int(expr_span.end as i64, span),
            };

            Value::record(record, expr_span)
        })
        .collect()
}

fn get_arguments(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &ast::Call,
    eval_expression_fn: fn(&EngineState, &mut Stack, &Expression) -> Result<Value, ShellError>,
) -> Vec<Value> {
    let working_set = StateWorkingSet::new(engine_state);
    let mut arg_value = vec![];
    let span = Span::test_data();
    for arg in &call.arguments {
        let (arg_type, expression) = match arg {
            // I think the second argument to Argument::Named is the short name, but I'm not really sure.
            // Please fix it if it's wrong. :)
            Argument::Named((name, short, opt_expr)) => {
                let arg_type = "named";
                let arg_value_name = name.item.clone();
                let arg_value_name_span_start = name.span.start as i64;
                let arg_value_name_span_end = name.span.end as i64;

                let record = record! {
                    "arg_type" => Value::string(arg_type, span),
                    "name" => Value::string(arg_value_name, name.span),
                    "type" => Value::string("string", span),
                    "span_start" => Value::int(arg_value_name_span_start, span),
                    "span_end" => Value::int(arg_value_name_span_end, span),
                };
                arg_value.push(Value::record(record, name.span));

                if let Some(shortcut) = short {
                    let arg_type = "short";
                    let arg_value_name = shortcut.item.clone();
                    let arg_value_name_span_start = shortcut.span.start as i64;
                    let arg_value_name_span_end = shortcut.span.end as i64;

                    let record = record! {
                        "arg_type" => Value::string(arg_type, span),
                        "name" => Value::string(arg_value_name, shortcut.span),
                        "type" => Value::string("string", span),
                        "span_start" => Value::int(arg_value_name_span_start, span),
                        "span_end" => Value::int(arg_value_name_span_end, span),
                    };
                    arg_value.push(Value::record(record, name.span));
                };

                match opt_expr {
                    Some(expression) => ("expr", expression),
                    None => continue,
                }
            }
            Argument::Positional(inner_expr) => ("positional", inner_expr),
            Argument::Unknown(inner_expr) => ("unknown", inner_expr),
            Argument::Spread(inner_expr) => ("spread", inner_expr),
        };
        arg_value.push(get_expression_record(
            &working_set,
            stack,
            arg_type,
            expression,
            eval_expression_fn,
        ));
    }

    arg_value
}

/// The row for an argument's expression: its value, or its source text and parsed type when it
/// holds a subexpression.
fn get_expression_record(
    working_set: &StateWorkingSet,
    stack: &mut Stack,
    arg_type: &str,
    expression: &Expression,
    eval_expression_fn: fn(&EngineState, &mut Stack, &Expression) -> Result<Value, ShellError>,
) -> Value {
    let engine_state = working_set.permanent_state;
    let span = Span::test_data();
    let (name, ty, value_span) = if has_subexpression(working_set, expression) {
        // A subexpression's block is compiled only as part of the closure, so it can't run on its
        // own. Compiling it here would run its commands, which `explain` mustn't do.
        let source = engine_state.get_span_contents(expression.span);
        (
            String::from_utf8_lossy(source).into_owned(),
            expression.ty.to_string(),
            expression.span,
        )
    } else {
        let value = get_expression_as_value(engine_state, stack, expression, eval_expression_fn);
        (
            debug_string_without_formatting(engine_state, &value),
            value.get_type().to_string(),
            value.span(),
        )
    };

    let record = record! {
        "arg_type" => Value::string(arg_type, span),
        "name" => Value::string(name, expression.span),
        "type" => Value::string(ty, span),
        "span_start" => Value::int(value_span.start as i64, span),
        "span_end" => Value::int(value_span.end as i64, span),
    };
    Value::record(record, expression.span)
}

/// Whether evaluating `expression` would reach a subexpression, as in `(1 + 2)`, `$"a(1)"` or
/// `[(ls)]`. A closure or block isn't run when it's evaluated, so its body doesn't count, and a
/// call doesn't either, because it reports its own error.
fn has_subexpression(working_set: &StateWorkingSet, expression: &Expression) -> bool {
    expression
        .find_map(working_set, &|expr| match expr.expr {
            Expr::Subexpression(_) => ControlFlow::Break(Some(())),
            Expr::Closure(_)
            | Expr::Block(_)
            | Expr::RowCondition(_)
            | Expr::MatchBlock(_)
            | Expr::Call(_)
            | Expr::ExternalCall(..) => ControlFlow::Break(None),
            _ => ControlFlow::Continue(()),
        })
        .is_some()
}

fn get_expression_as_value(
    engine_state: &EngineState,
    stack: &mut Stack,
    inner_expr: &Expression,
    eval_expression_fn: fn(&EngineState, &mut Stack, &Expression) -> Result<Value, ShellError>,
) -> Value {
    match eval_expression_fn(engine_state, stack, inner_expr) {
        Ok(v) => v,
        Err(error) => Value::error(error, inner_expr.span),
    }
}

pub fn debug_string_without_formatting(engine_state: &EngineState, value: &Value) -> String {
    match value {
        Value::Bool { val, .. } => val.to_string(),
        Value::Int { val, .. } => val.to_string(),
        Value::Float { val, .. } => val.to_string(),
        Value::Filesize { val, .. } => val.to_string(),
        Value::Duration { val, .. } => val.to_string(),
        Value::Date { val, .. } => format!("{val:?}"),
        Value::Range { val, .. } => val.to_string(),
        Value::String { val, .. } => val.clone(),
        Value::Glob { val, .. } => val.clone(),
        Value::List { vals: val, .. } => format!(
            "[{}]",
            val.iter()
                .map(|v| debug_string_without_formatting(engine_state, v))
                .collect::<Vec<_>>()
                .join(" ")
        ),
        Value::Record { val, .. } => format!(
            "{{{}}}",
            val.iter()
                .map(|(x, y)| format!(
                    "{}: {}",
                    x,
                    debug_string_without_formatting(engine_state, y)
                ))
                .collect::<Vec<_>>()
                .join(" ")
        ),
        Value::Closure { val, .. } => {
            let block = engine_state.try_get_block(val.block_id);
            if let Some(span) = block.and_then(|block| block.span) {
                let contents_bytes = engine_state.get_span_contents(span);
                let contents_string = String::from_utf8_lossy(contents_bytes);
                contents_string.to_string()
            } else {
                String::new()
            }
        }
        Value::Nothing { .. } => String::new(),
        Value::Error { error, .. } => format!("{error:?}"),
        Value::Binary { val, .. } => format!("{val:?}"),
        Value::CellPath { val, .. } => val.to_string(),
        // If we fail to collapse the custom value, just print <{type_name}> - failure is not
        // that critical here
        Value::Custom { val, .. } => val
            .to_base_value(value.span())
            .map(|val| debug_string_without_formatting(engine_state, &val))
            .unwrap_or_else(|_| format!("<{}>", val.type_name())),
    }
}
