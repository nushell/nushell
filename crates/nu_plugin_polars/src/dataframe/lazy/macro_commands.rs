/// Definition of multiple lazyframe commands using a macro rule
/// All of these commands have an identical body and only require
/// to have a change in the name, description and function
use crate::dataframe::values::{Column, NuDataFrame, NuLazyFrame};
use crate::{Cacheable, CustomValueSupport, PolarsPlugin};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, LabeledError, PipelineData, PluginExample, PluginSignature, ShellError, Span, Type,
    Value,
};

macro_rules! lazy_command {
    ($command: ident, $name: expr, $desc: expr, $examples: expr, $func: ident, $test: ident) => {
        #[derive(Clone)]
        pub struct $command;

        impl PluginCommand for $command {
            type Plugin = PolarsPlugin;

            fn signature(&self) -> PluginSignature {
                PluginSignature::build($name)
                    .usage($desc)
                    .input_output_type(
                        Type::Custom("dataframe".into()),
                        Type::Custom("dataframe".into()),
                    )
                    .category(Category::Custom("lazyframe".into()))
                    .plugin_examples($examples)
            }

            fn run(
                &self,
                plugin: &Self::Plugin,
                engine: &EngineInterface,
                call: &EvaluatedCall,
                input: PipelineData,
            ) -> Result<PipelineData, LabeledError> {
                let lazy = NuLazyFrame::try_from_pipeline(plugin, input, call.head)
                    .map_err(LabeledError::from)?;
                let lazy = NuLazyFrame::new(lazy.from_eager, lazy.into_polars().$func());

                Ok(PipelineData::Value(
                    lazy.cache(plugin, engine)?.into_value(call.head),
                    None,
                ))
            }
        }

        // fix tests
        // #[cfg(test)]
        // mod $test {
        //     use super::super::super::test_dataframe::test_dataframe;
        //     use super::*;
        //
        //     #[test]
        //     fn test_examples() {
        //         test_dataframe(vec![Box::new($command {})])
        //     }
        // }
    };

    ($command: ident, $name: expr, $desc: expr, $examples: expr, $func: ident, $test: ident, $ddot: expr) => {
        #[derive(Clone)]
        pub struct $command;

        impl PluginCommand for $command {
            type Plugin = PolarsPlugin;

            fn signature(&self) -> PluginSignature {
                PluginSignature::build($name)
                    .usage($desc)
                    .input_output_type(
                        Type::Custom("dataframe".into()),
                        Type::Custom("dataframe".into()),
                    )
                    .category(Category::Custom("lazyframe".into()))
                    .plugin_examples($examples)
            }

            fn run(
                &self,
                _plugin: &Self::Plugin,
                engine: &EngineInterface,
                call: &EvaluatedCall,
                input: PipelineData,
            ) -> Result<PipelineData, LabeledError> {
                let lazy =
                    NuLazyFrame::try_from_pipeline(input, call.head).map_err(LabeledError::from)?;
                let lazy = NuLazyFrame::new(lazy.from_eager, lazy.into_polars().$func($ddot));

                Ok(PipelineData::Value(
                    lazy.insert_cache(engine)?
                        .into_value(call.head)
                        .map_err(LabeledError::from)?,
                    None,
                ))
            }
        }

        // todo - fix tests
        // #[cfg(test)]
        // mod $test {
        //     use super::super::super::test_dataframe::test_dataframe;
        //     use super::*;
        //
        //     #[test]
        //     fn test_examples() {
        //         test_dataframe(vec![Box::new($command {})])
        //     }
        // }
    };

    ($command: ident, $name: expr, $desc: expr, $examples: expr, $func: ident?, $test: ident) => {
        #[derive(Clone)]
        pub struct $command;

        impl PluginCommand for $command {
            type Plugin = PolarsPlugin;

            fn signature(&self) -> PluginSignature {
                PluginSignature::build($name)
                    .usage($desc)
                    .input_output_type(
                        Type::Custom("dataframe".into()),
                        Type::Custom("dataframe".into()),
                    )
                    .category(Category::Custom("lazyframe".into()))
                    .plugin_examples($examples)
            }

            fn run(
                &self,
                plugin: &Self::Plugin,
                engine: &EngineInterface,
                call: &EvaluatedCall,
                input: PipelineData,
            ) -> Result<PipelineData, LabeledError> {
                let lazy = NuLazyFrame::try_from_pipeline(plugin, input, call.head)
                    .map_err(LabeledError::from)?;

                let lazy = NuLazyFrame::new(
                    lazy.from_eager,
                    lazy.into_polars()
                        .$func()
                        .map_err(|e| ShellError::GenericError {
                            error: "Dataframe Error".into(),
                            msg: e.to_string(),
                            help: None,
                            span: None,
                            inner: vec![],
                        })
                        .map_err(LabeledError::from)?,
                );

                Ok(PipelineData::Value(
                    lazy.cache(plugin, engine)?.into_value(call.head),
                    None,
                ))
            }
        }

        // todo - fix tests
        // #[cfg(test)]
        // mod $test {
        //     use super::super::super::test_dataframe::test_dataframe;
        //     use super::*;
        //
        //     #[test]
        //     fn test_examples() {
        //         test_dataframe(vec![Box::new($command {})])
        //     }
        // }
    };
}

// LazyReverse command
// Expands to a command definition for reverse
lazy_command!(
    LazyReverse,
    "polars reverse",
    "Reverses the LazyFrame",
    vec![PluginExample {
        description: "Reverses the dataframe.".into(),
        example: "[[a b]; [6 2] [4 2] [2 2]] | polars into-df | polars reverse".into(),
        result: Some(
            NuDataFrame::try_from_columns(
                vec![
                    Column::new(
                        "a".to_string(),
                        vec![Value::test_int(2), Value::test_int(4), Value::test_int(6),],
                    ),
                    Column::new(
                        "b".to_string(),
                        vec![Value::test_int(2), Value::test_int(2), Value::test_int(2),],
                    ),
                ],
                None
            )
            .expect("simple df for test should not fail")
            .into_value(Span::test_data()),
        ),
    },],
    reverse,
    test_reverse
);

// LazyCache command
// Expands to a command definition for cache
lazy_command!(
    LazyCache,
    "polars cache",
    "Caches operations in a new LazyFrame.",
    vec![PluginExample {
        description: "Caches the result into a new LazyFrame".into(),
        example: "[[a b]; [6 2] [4 2] [2 2]] | polars into-df | polars reverse | polars cache"
            .into(),
        result: None,
    }],
    cache,
    test_cache
);

// LazyMedian command
// Expands to a command definition for median aggregation
lazy_command!(
    LazyMedian,
    "polars median",
    "Aggregates columns to their median value",
    vec![PluginExample {
        description: "Median value from columns in a dataframe".into(),
        example: "[[a b]; [6 2] [4 2] [2 2]] | polars into-df | polars median".into(),
        result: Some(
            NuDataFrame::try_from_columns(
                vec![
                    Column::new("a".to_string(), vec![Value::test_float(4.0)],),
                    Column::new("b".to_string(), vec![Value::test_float(2.0)],),
                ],
                None
            )
            .expect("simple df for test should not fail")
            .into_value(Span::test_data()),
        ),
    },],
    median?,
    test_median
);
