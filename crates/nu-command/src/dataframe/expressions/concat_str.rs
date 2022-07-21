use crate::dataframe::values::{Column, NuDataFrame, NuExpression};
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};
use polars::prelude::concat_str;

#[derive(Clone)]
pub struct ExprConcatStr;

impl Command for ExprConcatStr {
    fn name(&self) -> &str {
        "concat-str"
    }

    fn usage(&self) -> &str {
        "Creates a concat string expression"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "separator",
                SyntaxShape::String,
                "Separator used during the concatenation",
            )
            .required(
                "concat expressions",
                SyntaxShape::List(Box::new(SyntaxShape::Any)),
                "Expression(s) that define the string concatenation",
            )
            .input_type(Type::Any)
            .output_type(Type::Custom("expression".into()))
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Creates a concat string expression",
            example: r#"let df = ([[a b c]; [one two 1] [three four 2]] | into df);
    $df | with-column ((concat-str "-" [(col a) (col b) ((col c) * 2)]) | as concat)"#,
            result: Some(
                NuDataFrame::try_from_columns(vec![
                    Column::new(
                        "a".to_string(),
                        vec![Value::test_string("one"), Value::test_string("three")],
                    ),
                    Column::new(
                        "b".to_string(),
                        vec![Value::test_string("two"), Value::test_string("four")],
                    ),
                    Column::new(
                        "c".to_string(),
                        vec![Value::test_int(1), Value::test_int(2)],
                    ),
                    Column::new(
                        "concat".to_string(),
                        vec![
                            Value::test_string("one-two-2"),
                            Value::test_string("three-four-4"),
                        ],
                    ),
                ])
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let separator: String = call.req(engine_state, stack, 0)?;
        let value: Value = call.req(engine_state, stack, 1)?;

        let expressions = NuExpression::extract_exprs(value)?;
        let expr: NuExpression = concat_str(expressions, &separator).into();

        Ok(PipelineData::Value(expr.into_value(call.head), None))
    }
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;
    use crate::dataframe::eager::WithColumn;
    use crate::dataframe::expressions::alias::ExprAlias;
    use crate::dataframe::expressions::col::ExprCol;

    #[test]
    fn test_examples() {
        test_dataframe(vec![
            Box::new(ExprConcatStr {}),
            Box::new(ExprAlias {}),
            Box::new(ExprCol {}),
            Box::new(WithColumn {}),
        ])
    }
}
