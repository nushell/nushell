use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Type,
};

use super::explode::explode;

#[derive(Clone)]
pub struct LazyFlatten;

impl Command for LazyFlatten {
    fn name(&self) -> &str {
        "dfr flatten"
    }

    fn usage(&self) -> &str {
        "An alias for dfr explode"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .optional(
                "columns",
                SyntaxShape::String,
                "columns to explode, only applicable for dataframes",
            )
            .input_output_types(vec![
                (
                    Type::Custom("expression".into()),
                    Type::Custom("expression".into()),
                ),
                (
                    Type::Custom("dataframe".into()),
                    Type::Custom("dataframe".into()),
                ),
            ])
            .category(Category::Custom("lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Flatten the specified dataframe",
                example: "",
                result: None,
            },
            Example {
                description: "todo expression case",
                example: "",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        explode(call, input)
    }
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::{build_test_engine_state, test_dataframe_example};
    use super::*;
    use crate::dataframe::lazy::aggregate::LazyAggregate;
    use crate::dataframe::lazy::groupby::ToLazyGroupBy;

    #[test]
    fn test_examples_dataframe() {
        let mut engine_state = build_test_engine_state(vec![Box::new(LazyFlatten {})]);
        test_dataframe_example(&mut engine_state, &LazyFlatten.examples()[0]);
        test_dataframe_example(&mut engine_state, &LazyFlatten.examples()[1]);
    }

    #[test]
    fn test_examples_expression() {
        let mut engine_state = build_test_engine_state(vec![
            Box::new(LazyFlatten {}),
            Box::new(LazyAggregate {}),
            Box::new(ToLazyGroupBy {}),
        ]);
        test_dataframe_example(&mut engine_state, &LazyFlatten.examples()[2]);
    }
}
