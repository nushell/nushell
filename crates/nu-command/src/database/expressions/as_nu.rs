use crate::database::values::dsl::{ExprDb, SelectDb};

use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Type, Value,
};

#[derive(Clone)]
pub struct ExprAsNu;

impl Command for ExprAsNu {
    fn name(&self) -> &str {
        "into nu"
    }

    fn usage(&self) -> &str {
        "Convert a db expression into a nu value for access and exploration"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_type(Type::Custom("db-expression".into()))
            .output_type(Type::Any)
            .category(Category::Custom("db-expression".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Convert a col expression into a nushell value",
            example: "field name_1 | into nu",
            result: Some(Value::Record {
                cols: vec!["value".into(), "quoted_style".into()],
                vals: vec![
                    Value::String {
                        val: "name_1".into(),
                        span: Span::test_data(),
                    },
                    Value::String {
                        val: "None".into(),
                        span: Span::test_data(),
                    },
                ],
                span: Span::test_data(),
            }),
        }]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let value = input.into_value(call.head);
        if let Ok(expr) = ExprDb::try_from_value(&value) {
            Ok(expr.to_value(call.head).into_pipeline_data())
        } else {
            let select = SelectDb::try_from_value(&value)?;
            Ok(select.to_value(call.head).into_pipeline_data())
        }
    }
}

#[cfg(test)]
mod test {
    use super::super::FieldExpr;
    use super::*;
    use crate::database::test_database::test_database;

    #[test]
    fn test_examples() {
        test_database(vec![Box::new(ExprAsNu {}), Box::new(FieldExpr {})])
    }
}
