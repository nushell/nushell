use crate::dataframe::values::NuExpression;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};
use polars::prelude::arg_where;

#[derive(Clone)]
pub struct ExprArgWhere;

impl Command for ExprArgWhere {
    fn name(&self) -> &str {
        "arg-where"
    }

    fn usage(&self) -> &str {
        "Creates an expression that returns the arguments where expression is true"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("column name", SyntaxShape::Any, "Expression to evaluate")
            .input_type(Type::Any)
            .output_type(Type::Custom("expression".into()))
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "",
            example: "",
            result: Some(Value::Nothing {
                span: Span::test_data(),
            }),
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let value: Value = call.req(engine_state, stack, 0)?;
        let expr = NuExpression::try_from_value(value)?;
        let expr: NuExpression = arg_where(expr.into_polars()).into();

        Ok(PipelineData::Value(expr.into_value(call.head), None))
    }
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;
    use crate::dataframe::expressions::as_nu::ExprAsNu;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(ExprArgWhere {}), Box::new(ExprAsNu {})])
    }
}
