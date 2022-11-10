use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math eval"
    }

    fn usage(&self) -> &str {
        "Evaluate a math expression into a number"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["evaluation", "solve", "equation", "expression"]
    }

    fn signature(&self) -> Signature {
        Signature::build("math eval")
            .input_output_types(vec![(Type::String, Type::Number)])
            .optional(
                "math expression",
                SyntaxShape::String,
                "the math expression to evaluate",
            )
            .category(Category::Math)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let spanned_expr: Option<Spanned<String>> = call.opt(engine_state, stack, 0)?;
        eval(spanned_expr, input, engine_state, call.head)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Evaluate math in the pipeline",
            example: "'10 / 4' | math eval",
            result: Some(Value::Float {
                val: 2.5,
                span: Span::test_data(),
            }),
        }]
    }
}

pub fn eval(
    spanned_expr: Option<Spanned<String>>,
    input: PipelineData,
    engine_state: &EngineState,
    head: Span,
) -> Result<PipelineData, ShellError> {
    if let Some(expr) = spanned_expr {
        match parse(&expr.item, &expr.span) {
            Ok(value) => Ok(PipelineData::Value(value, None)),
            Err(err) => Err(ShellError::UnsupportedInput(
                format!("Math evaluation error: {}", err),
                expr.span,
            )),
        }
    } else {
        if let PipelineData::Value(Value::Nothing { .. }, ..) = input {
            return Ok(input);
        }
        input.map(
            move |val| {
                if let Ok(string) = val.as_string() {
                    match parse(&string, &val.span().unwrap_or(head)) {
                        Ok(value) => value,
                        Err(err) => Value::Error {
                            error: ShellError::UnsupportedInput(
                                format!("Math evaluation error: {}", err),
                                val.span().unwrap_or(head),
                            ),
                        },
                    }
                } else {
                    Value::Error {
                        error: ShellError::UnsupportedInput(
                            "Expected a string from pipeline".to_string(),
                            val.span().unwrap_or(head),
                        ),
                    }
                }
            },
            engine_state.ctrlc.clone(),
        )
    }
}

pub fn parse(math_expression: &str, span: &Span) -> Result<Value, String> {
    let mut ctx = meval::Context::new();
    ctx.var("tau", std::f64::consts::TAU);
    match meval::eval_str_with_context(math_expression, &ctx) {
        Ok(num) if num.is_infinite() || num.is_nan() => Err("cannot represent result".to_string()),
        Ok(num) => Ok(Value::Float {
            val: num,
            span: *span,
        }),
        Err(error) => Err(error.to_string().to_lowercase()),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
