use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};
use polars::frame::explode::MeltArgs;

use crate::{
    dataframe::values::utils::convert_columns_string,
    values::{CustomValueSupport, NuLazyFrame},
    PolarsPlugin,
};

use super::super::values::{Column, NuDataFrame};

#[derive(Clone)]
pub struct MeltDF;

impl PluginCommand for MeltDF {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars melt"
    }

    fn usage(&self) -> &str {
        "Unpivot a DataFrame from wide to long format."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required_named(
                "columns",
                SyntaxShape::Table(vec![]),
                "column names for melting",
                Some('c'),
            )
            .required_named(
                "values",
                SyntaxShape::Table(vec![]),
                "column names used as value columns",
                Some('v'),
            )
            .named(
                "variable-name",
                SyntaxShape::String,
                "optional name for variable column",
                Some('r'),
            )
            .named(
                "value-name",
                SyntaxShape::String,
                "optional name for value column",
                Some('l'),
            )
            .switch(
                "streamable",
                "Use polar's streaming engine. Results will not have a stable ordering.",
                Some('s'),
            )
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "melt dataframe",
            example:
                "[[a b c d]; [x 1 4 a] [y 2 5 b] [z 3 6 c]] | polars into-df | polars melt -c [b c] -v [a d] | polars collect",
            result: Some(
                NuDataFrame::try_from_columns(vec![
                    Column::new(
                        "b".to_string(),
                        vec![
                            Value::test_int(1),
                            Value::test_int(2),
                            Value::test_int(3),
                            Value::test_int(1),
                            Value::test_int(2),
                            Value::test_int(3),
                        ],
                    ),
                    Column::new(
                        "c".to_string(),
                        vec![
                            Value::test_int(4),
                            Value::test_int(5),
                            Value::test_int(6),
                            Value::test_int(4),
                            Value::test_int(5),
                            Value::test_int(6),
                        ],
                    ),
                    Column::new(
                        "variable".to_string(),
                        vec![
                            Value::test_string("a"),
                            Value::test_string("a"),
                            Value::test_string("a"),
                            Value::test_string("d"),
                            Value::test_string("d"),
                            Value::test_string("d"),
                        ],
                    ),
                    Column::new(
                        "value".to_string(),
                        vec![
                            Value::test_string("x"),
                            Value::test_string("y"),
                            Value::test_string("z"),
                            Value::test_string("a"),
                            Value::test_string("b"),
                            Value::test_string("c"),
                        ],
                    ),
                ], None)
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        }]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        command(plugin, engine, call, input).map_err(LabeledError::from)
    }
}

fn command(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let id_col: Vec<Value> = call.get_flag("columns")?.expect("required value");
    let val_col: Vec<Value> = call.get_flag("values")?.expect("required value");

    let value_name = call.get_flag("value-name")?.map(|v: String| v.into());
    let variable_name = call.get_flag("variable-name")?.map(|v: String| v.into());
    let streamable = call.has_flag("streamable")?;

    let (id_vars, _id_col_span) = convert_columns_string(id_col, call.head)?;
    let id_vars = id_vars.into_iter().map(Into::into).collect();
    let (value_vars, _val_col_span) = convert_columns_string(val_col, call.head)?;
    let value_vars = value_vars.into_iter().map(Into::into).collect();

    let df = NuLazyFrame::try_from_pipeline_coerce(plugin, input, call.head)?;
    let polars_df = df.to_polars();

    let args = MeltArgs {
        id_vars,
        value_vars,
        variable_name,
        value_name,
        streamable,
    };

    let res = polars_df.melt(args);
    let res = NuLazyFrame::new(res);
    res.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use crate::test::test_polars_plugin_command;

    use super::*;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&MeltDF)
    }
}
