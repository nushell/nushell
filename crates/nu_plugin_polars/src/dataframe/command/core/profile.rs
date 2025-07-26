use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Type, Value, record,
};

use crate::{
    PolarsPlugin,
    values::{
        CustomValueSupport, NuDataFrame, NuLazyFrame, PolarsPluginObject, PolarsPluginType,
        cant_convert_err,
    },
};

pub struct ProfileDF;

impl PluginCommand for ProfileDF {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars profile"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn description(&self) -> &str {
        "Profile a lazy dataframe."
    }

    fn extra_description(&self) -> &str {
        r#"This will run the query and return a record containing the materialized DataFrame and a DataFrame that contains profiling information of each node that is executed.

The units of the timings are microseconds."#
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Profile a lazy dataframe",
            example: r#"[[a b]; [1 2] [1 4] [2 6] [2 4]]
    | polars into-lazy
    | polars group-by a
    | polars agg [
        (polars col b | polars min | polars as "b_min")
        (polars col b | polars max | polars as "b_max")
        (polars col b | polars sum | polars as "b_sum")
     ]
    | polars profile
"#,
            result: None,
        }]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.metadata();
        let value = input.into_value(call.head)?;
        match PolarsPluginObject::try_from_value(plugin, &value)? {
            PolarsPluginObject::NuDataFrame(df) => command_lazy(plugin, engine, call, df.lazy()),
            PolarsPluginObject::NuLazyFrame(lazy) => command_lazy(plugin, engine, call, lazy),
            _ => Err(cant_convert_err(
                &value,
                &[PolarsPluginType::NuDataFrame, PolarsPluginType::NuLazyFrame],
            )),
        }
        .map_err(LabeledError::from)
        .map(|pd| pd.set_metadata(metadata))
    }
}

fn command_lazy(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    lazy: NuLazyFrame,
) -> Result<PipelineData, ShellError> {
    let (df, profiling_df) = lazy
        .to_polars()
        .profile()
        .map_err(|e| ShellError::GenericError {
            error: format!("Could not profile dataframe: {e}"),
            msg: "".into(),
            span: Some(call.head),
            help: None,
            inner: vec![],
        })?;

    let df = NuDataFrame::from(df).cache_and_to_value(plugin, engine, call.head)?;
    let profiling_df =
        NuDataFrame::from(profiling_df).cache_and_to_value(plugin, engine, call.head)?;

    let result = Value::record(
        record!(
            "dataframe" => df,
            "profiling" => profiling_df,
        ),
        call.head,
    );

    Ok(PipelineData::value(result, None))
}
