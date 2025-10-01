use crate::{
    PolarsPlugin,
    values::{CustomValueSupport, NuLazyFrame},
};

use crate::values::NuDataFrame;
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};
use polars::{
    df,
    prelude::{LazyFrame, UnionArgs},
};

#[derive(Clone)]
pub struct ConcatDF;

impl PluginCommand for ConcatDF {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars concat"
    }

    fn description(&self) -> &str {
        "Concatenate two or more dataframes."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .switch("no-parallel", "Disable parallel execution", None)
            .switch("rechunk", "Rechunk the resulting dataframe", None)
            .switch("to-supertypes", "Cast to supertypes", None)
            .switch("diagonal", "Concatenate dataframes diagonally", None)
            .switch(
                "no-maintain-order",
                "Do not maintain order. The default behavior is to maintain order.",
                None,
            )
            .switch(
                "from-partitioned-ds",
                "Concatenate dataframes from a partitioned dataset",
                None,
            )
            .rest(
                "dataframes",
                SyntaxShape::Any,
                "The dataframes to concatenate",
            )
            .input_output_type(Type::Any, Type::Custom("dataframe".into()))
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Concatenates two dataframes with the dataframe in the pipeline.",
                example: "[[a b]; [1 2]] | polars into-df 
                    | polars concat ([[a b]; [3 4]] | polars into-df) ([[a b]; [5 6]] | polars into-df) 
                    | polars collect 
                    | polars sort-by [a b]",
                result: Some(
                    NuDataFrame::from(
                        df!(
                            "a" => [1, 3, 5],
                            "b" => [2, 4, 6],
                        )
                        .expect("simple df for test should not fail"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Concatenates three dataframes together",
                example: "polars concat ([[a b]; [1 2]] | polars into-df) ([[a b]; [3 4]] | polars into-df) ([[a b]; [5 6]] | polars into-df) 
                    | polars collect 
                    | polars sort-by [a b]",
                result: Some(
                    NuDataFrame::from(
                        df!(
                            "a" => [1, 3, 5],
                            "b" => [2, 4, 6],
                        )
                        .expect("simple df for test should not fail"),
                    )
                    .into_value(Span::test_data()),
                ),
            }
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
        let maybe_df = NuLazyFrame::try_from_pipeline_coerce(plugin, input, call.head).ok();
        command_lazy(plugin, engine, call, maybe_df)
            .map_err(LabeledError::from)
            .map(|pd| pd.set_metadata(metadata))
    }
}

fn command_lazy(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    maybe_lazy: Option<NuLazyFrame>,
) -> Result<PipelineData, ShellError> {
    let parallel = !call.has_flag("no-parallel")?;
    let rechunk = call.has_flag("rechunk")?;
    let to_supertypes = call.has_flag("to-supertypes")?;
    let diagonal = call.has_flag("diagonal")?;
    let from_partitioned_ds = call.has_flag("from-partitioned-ds")?;
    let maintain_order = !call.has_flag("no-maintain-order")?;
    let mut dataframes = call
        .rest::<Value>(0)?
        .iter()
        .map(|v| NuLazyFrame::try_from_value_coerce(plugin, v).map(|lazy| lazy.to_polars()))
        .collect::<Result<Vec<LazyFrame>, ShellError>>()?;

    if dataframes.is_empty() {
        Err(ShellError::GenericError {
            error: "At least one other dataframe must be provided".into(),
            msg: "".into(),
            span: Some(call.head),
            help: None,
            inner: vec![],
        })
    } else {
        if let Some(lazy) = maybe_lazy.as_ref() {
            dataframes.insert(0, lazy.to_polars());
        }
        let args = UnionArgs {
            parallel,
            rechunk,
            to_supertypes,
            diagonal,
            from_partitioned_ds,
            maintain_order,
        };

        let res: NuLazyFrame = polars::prelude::concat(&dataframes, args)
            .map_err(|e| ShellError::GenericError {
                error: format!("Failed to concatenate dataframes: {e}"),
                msg: "".into(),
                span: Some(call.head),
                help: None,
                inner: vec![],
            })?
            .into();

        res.to_pipeline_data(plugin, engine, call.head)
    }
}

#[cfg(test)]
mod test {
    use crate::test::test_polars_plugin_command;

    use super::*;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&ConcatDF)
    }
}
