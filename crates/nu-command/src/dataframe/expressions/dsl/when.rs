use crate::dataframe::values::NuExpression;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use polars::prelude::when;

#[derive(Clone)]
pub struct ExprWhen;

impl Command for ExprWhen {
    fn name(&self) -> &str {
        "dfr when"
    }

    fn usage(&self) -> &str {
        "Creates a when expression"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "when predicate",
                SyntaxShape::Any,
                "Name of column to be used",
            )
            .required_named(
                "then",
                SyntaxShape::Any,
                "Expression that will be applied when predicate is true",
                Some('t'),
            )
            .required_named(
                "otherwise",
                SyntaxShape::Any,
                "Expression that will be applied when predicate is false",
                Some('o'),
            )
            .category(Category::Custom("expressions".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Create a new column for the dataframe",
            example: r#"[[a b]; [1 2] [3 4]]
   | dfr to-df
   | dfr to-lazy
   | dfr with-column (
       dfr when ((dfr col a) > 2) --then 4 --otherwise 5  | dfr as "c"
     )
   | dfr collect"#,
            result: None,
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let predicate: Value = call.req(engine_state, stack, 0)?;
        let predicate = NuExpression::try_from_value(predicate)?;

        let then: Value = call
            .get_flag(engine_state, stack, "then")?
            .expect("it is a required named value");
        let then = NuExpression::try_from_value(then)?;
        let otherwise: Value = call
            .get_flag(engine_state, stack, "otherwise")?
            .expect("it is a required named value");
        let otherwise = NuExpression::try_from_value(otherwise)?;

        let expr: NuExpression = when(predicate.into_polars())
            .then(then.into_polars())
            .otherwise(otherwise.into_polars())
            .into();

        Ok(PipelineData::Value(expr.into_value(call.head), None))
    }
}

#[cfg(test)]
mod test {
    use super::super::super::super::test_dataframe::test_dataframe;
    use super::super::super::ExprToNu;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(ExprWhen {}), Box::new(ExprToNu {})])
    }
}
