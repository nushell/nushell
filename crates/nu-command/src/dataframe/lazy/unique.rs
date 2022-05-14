use crate::dataframe::values::NuLazyFrame;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use polars::prelude::UniqueKeepStrategy;

use super::utils::extract_strings;

#[derive(Clone)]
pub struct LazyUnique;

impl Command for LazyUnique {
    fn name(&self) -> &str {
        "dfr unique"
    }

    fn usage(&self) -> &str {
        "Drops duplicate rows from lazyframe"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .named(
                "subset",
                SyntaxShape::Any,
                "Subset of column(s) to use to maintain rows",
                Some('s'),
            )
            .switch(
                "last",
                "Keeps last unique value. Default keeps first value",
                Some('l'),
            )
            .switch(
                "maintain-order",
                "Keep the same order as the original DataFrame",
                Some('k'),
            )
            .category(Category::Custom("lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "",
            example: "",
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
        let last = call.has_flag("last");
        let maintain = call.has_flag("maintain-order");

        let subset: Option<Value> = call.get_flag(engine_state, stack, "subset")?;
        let subset = match subset {
            Some(value) => Some(extract_strings(value)?),
            None => None,
        };

        let strategy = if last {
            UniqueKeepStrategy::Last
        } else {
            UniqueKeepStrategy::First
        };

        let lazy = NuLazyFrame::try_from_pipeline(input, call.head)?.into_polars();
        let lazy: NuLazyFrame = if maintain {
            lazy.unique(subset, strategy).into()
        } else {
            lazy.unique_stable(subset, strategy).into()
        };

        Ok(PipelineData::Value(lazy.into_value(call.head), None))
    }
}

//#[cfg(test)]
//mod test {
//    use super::super::super::test_dataframe::test_dataframe;
//    use super::*;
//
//    #[test]
//    fn test_examples() {
//        test_dataframe(vec![Box::new(LazyUnique {})])
//    }
//}
