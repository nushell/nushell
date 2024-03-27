use crate::dataframe::values::NuExpression;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct ExprAlias;

impl Command for ExprAlias {
    fn name(&self) -> &str {
        "dfr as"
    }

    fn usage(&self) -> &str {
        "Creates an alias expression."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "Alias name",
                SyntaxShape::String,
                "Alias name for the expression",
            )
            .input_output_type(
                Type::Custom("expression".into()),
                Type::Custom("expression".into()),
            )
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Creates and alias expression",
            example: "dfr col a | dfr as new_a | dfr into-nu",
            result: {
                let record = Value::test_record(record! {
                    "expr" =>  Value::test_record(record! {
                        "expr" =>  Value::test_string("column"),
                        "value" => Value::test_string("a"),
                    }),
                    "alias" => Value::test_string("new_a"),
                });

                Some(record)
            },
        }]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["aka", "abbr", "otherwise"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let alias: String = call.req(engine_state, stack, 0)?;

        let expr = NuExpression::try_from_pipeline(input, call.head)?;
        let expr: NuExpression = expr.into_polars().alias(alias.as_str()).into();

        Ok(PipelineData::Value(
            NuExpression::into_value(expr, call.head),
            None,
        ))
    }
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;
    use crate::dataframe::eager::ToNu;
    use crate::dataframe::expressions::ExprCol;

    #[test]
    fn test_examples() {
        test_dataframe(vec![
            Box::new(ExprAlias {}),
            Box::new(ExprCol {}),
            Box::new(ToNu {}),
        ])
    }
}
