use crate::dataframe::values::NuSchema;

use super::super::values::{NuDataFrame, NuLazyFrame};

use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct ToLazyFrame;

impl Command for ToLazyFrame {
    fn name(&self) -> &str {
        "dfr into-lazy"
    }

    fn usage(&self) -> &str {
        "Converts a dataframe into a lazy dataframe."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .named(
                "schema",
                SyntaxShape::Record(vec![]),
                r#"Polars Schema in format [{name: str}]. CSV, JSON, and JSONL files"#,
                Some('s'),
            )
            .input_output_type(Type::Any, Type::Custom("dataframe".into()))
            .category(Category::Custom("lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Takes a dictionary and creates a lazy dataframe",
            example: "[[a b];[1 2] [3 4]] | dfr into-lazy",
            result: None,
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let maybe_schema = call
            .get_flag(engine_state, stack, "schema")?
            .map(|schema| NuSchema::try_from(&schema))
            .transpose()?;

        let df = NuDataFrame::try_from_iter(input.into_iter(), maybe_schema)?;
        let lazy = NuLazyFrame::from_dataframe(df);
        let value = Value::custom_value(Box::new(lazy), call.head);

        Ok(PipelineData::Value(value, None))
    }
}
