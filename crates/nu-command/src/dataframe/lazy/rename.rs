use crate::dataframe::values::NuLazyFrame;

use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, FromValue, PipelineData, ShellError, Signature, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct LazyRename;

impl Command for LazyRename {
    fn name(&self) -> &str {
        "dfl rename"
    }

    fn usage(&self) -> &str {
        "Renames columns from lazyframe"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "columns",
                SyntaxShape::Any,
                "Column(s) to be renamed. A string or list of strings",
            )
            .required(
                "new names",
                SyntaxShape::Any,
                "New names for the selected column(s). A string or list of strings",
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
        let columns: Value = call.req(engine_state, stack, 0)?;
        let columns = extract_names(columns)?;

        let new_names: Value = call.req(engine_state, stack, 1)?;
        let new_names = extract_names(new_names)?;

        if columns.len() != new_names.len() {
            let value: Value = call.req(engine_state, stack, 1)?;
            return Err(ShellError::IncompatibleParametersSingle(
                "New name list has different size to column list".into(),
                value.span()?,
            ));
        }

        let lazy = NuLazyFrame::try_from_pipeline(input, call.head)?.into_polars();
        let lazy: NuLazyFrame = lazy.rename(&columns, &new_names).into();

        Ok(PipelineData::Value(lazy.into_value(call.head), None))
    }
}

fn extract_names(value: Value) -> Result<Vec<String>, ShellError> {
    match (
        <String as FromValue>::from_value(&value),
        <Vec<String> as FromValue>::from_value(&value),
    ) {
        (Ok(col), Err(_)) => Ok(vec![col]),
        (Err(_), Ok(cols)) => Ok(cols),
        _ => Err(ShellError::IncompatibleParametersSingle(
            "Expected a string or list of strings".into(),
            value.span()?,
        )),
    }
}

//#[cfg(test)]
//mod test {
//    use super::super::super::test_dataframe::test_dataframe;
//    use super::*;
//
//    #[test]
//    fn test_examples() {
//        test_dataframe(vec![Box::new(LazyRename {})])
//    }
//}
