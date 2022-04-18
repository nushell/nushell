use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, PipelineData, ShellError, Signature, SyntaxShape,
};
use polars::prelude::LazyCsvReader;
use crate::dataframe::values::NuLazyFrame;

#[derive(Clone)]
pub struct LazyOpenCSV;

impl Command for LazyOpenCSV {
    fn name(&self) -> &str {
        "dfl open-csv"
    }

    fn usage(&self) -> &str {
        "Creates a lazyframe from a CSV file"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("path", SyntaxShape::String, "CSV file path")
            .category(Category::Custom("lazyframe".into()))
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let path: String = call.req(engine_state, stack, 0)?;
        
        let reader = LazyCsvReader::new(path);
        
        
        let lazy: NuLazyFrame = reader
            .finish()
            .expect("change to general error once merged")
            .into();
        
        Ok(PipelineData::Value(lazy.into_value(call.head), None))
    }
}
