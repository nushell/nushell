use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape,
};
use polars::{df, prelude::PlSmallStr};

use crate::{
    PolarsPlugin,
    values::{CustomValueSupport, NuLazyFrame, PolarsPluginObject, PolarsPluginType},
};

use crate::values::NuDataFrame;

#[derive(Clone)]
pub struct UnnestDF;

impl PluginCommand for UnnestDF {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars unnest"
    }

    fn description(&self) -> &str {
        "Decompose struct columns into separate columns for each of their fields. The new columns will be inserted into the dataframe at the location of the struct column."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .named(
                "separator",
                SyntaxShape::String,
                "optional separator to use when creating new column names",
                Some('s'),
            )
            .rest("cols", SyntaxShape::String, "columns to unnest")
            .input_output_types(vec![
                (
                    PolarsPluginType::NuDataFrame.into(),
                    PolarsPluginType::NuDataFrame.into(),
                ),
                (
                    PolarsPluginType::NuLazyFrame.into(),
                    PolarsPluginType::NuLazyFrame.into(),
                ),
            ])
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Unnest a dataframe",
                example: r#"[[id person]; [1 {name: "Bob", age: 36}] [2 {name: "Betty", age: 63}]] 
                    | polars into-df -s {id: i64, person: {name: str, age: u8}} 
                    | polars unnest person
                    | polars get id name age
                    | polars sort-by id"#,
                result: Some(
                    NuDataFrame::from(
                        df!(
                            "id" => [1, 2],
                            "name" => ["Bob", "Betty"],
                            "age" => [36, 63]
                        )
                        .expect("Should be able to create a simple dataframe"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Unnest a lazy dataframe",
                example: r#"[[id person]; [1 {name: "Bob", age: 36}] [2 {name: "Betty", age: 63}]] 
                    | polars into-df -s {id: i64, person: {name: str, age: u8}} 
                    | polars into-lazy 
                    | polars unnest person
                    | polars select (polars col id) (polars col name) (polars col age)
                    | polars collect
                    | polars sort-by id"#,
                result: Some(
                    NuDataFrame::from(
                        df!(
                            "id" => [1, 2],
                            "name" => ["Bob", "Betty"],
                            "age" => [36, 63]
                        )
                        .expect("Should be able to create a simple dataframe"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Unnest with a custom separator",
                example: r#"[[id person]; [1 {name: "Bob", age: 36}] [2 {name: "Betty", age: 63}]] 
                    | polars into-df -s {id: i64, person: {name: str, age: u8}} 
                    | polars unnest person -s "_"
                    | polars get id person_name person_age
                    | polars sort-by id"#,
                result: Some(
                    NuDataFrame::from(
                        df!(
                            "id" => [1, 2],
                            "person_name" => ["Bob", "Betty"],
                            "person_age" => [36, 63]
                        )
                        .expect("Should be able to create a simple dataframe"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
        ]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.metadata();
        match PolarsPluginObject::try_from_pipeline(plugin, input, call.head)? {
            PolarsPluginObject::NuDataFrame(df) => command_eager(plugin, engine, call, df),
            PolarsPluginObject::NuLazyFrame(lazy) => command_lazy(plugin, engine, call, lazy),
            _ => Err(ShellError::GenericError {
                error: "Must be a dataframe or lazy dataframe".into(),
                msg: "".into(),
                span: Some(call.head),
                help: None,
                inner: vec![],
            }),
        }
        .map_err(LabeledError::from)
        .map(|pd| pd.set_metadata(metadata))
    }
}

fn command_eager(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    df: NuDataFrame,
) -> Result<PipelineData, ShellError> {
    let cols = call.rest::<String>(0)?;
    let separator = call.get_flag::<String>("separator")?;
    let polars = df.to_polars();
    let result: NuDataFrame = polars
        .unnest(cols, separator.as_deref())
        .map_err(|e| ShellError::GenericError {
            error: format!("Error unnesting dataframe: {e}"),
            msg: "".into(),
            span: Some(call.head),
            help: None,
            inner: vec![],
        })?
        .into();
    result.to_pipeline_data(plugin, engine, call.head)
}

fn command_lazy(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    df: NuLazyFrame,
) -> Result<PipelineData, ShellError> {
    let cols = call.rest::<String>(0)?;
    let separator = call.get_flag::<String>("separator")?.map(PlSmallStr::from);

    let polars = df.to_polars();
    // todo - allow selectors to be passed in here
    let result: NuLazyFrame = polars.unnest(polars::prelude::cols(cols), separator).into();
    result.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use crate::test::test_polars_plugin_command;

    use super::*;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&UnnestDF)
    }
}
