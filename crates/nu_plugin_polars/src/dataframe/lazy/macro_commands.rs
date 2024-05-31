/// Definition of multiple lazyframe commands using a macro rule
/// All of these commands have an identical body and only require
/// to have a change in the name, description and function
use crate::dataframe::values::{Column, NuDataFrame, NuLazyFrame};
use crate::values::CustomValueSupport;
use crate::PolarsPlugin;
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, Example, LabeledError, PipelineData, Signature, Span, Type, Value};

macro_rules! lazy_command {
    ($command: ident, $name: expr, $desc: expr, $examples: expr, $func: ident, $test: ident) => {
        #[derive(Clone)]
        pub struct $command;

        impl PluginCommand for $command {
            type Plugin = PolarsPlugin;

            fn name(&self) -> &str {
                $name
            }

            fn usage(&self) -> &str {
                $desc
            }

            fn signature(&self) -> Signature {
                Signature::build(self.name())
                    .usage($desc)
                    .input_output_type(
                        Type::Custom("dataframe".into()),
                        Type::Custom("dataframe".into()),
                    )
                    .category(Category::Custom("lazyframe".into()))
            }

            fn examples(&self) -> Vec<Example> {
                $examples
            }

            fn run(
                &self,
                plugin: &Self::Plugin,
                engine: &EngineInterface,
                call: &EvaluatedCall,
                input: PipelineData,
            ) -> Result<PipelineData, LabeledError> {
                let lazy = NuLazyFrame::try_from_pipeline_coerce(plugin, input, call.head)
                    .map_err(LabeledError::from)?;
                let lazy = NuLazyFrame::new(lazy.from_eager, lazy.to_polars().$func());
                lazy.to_pipeline_data(plugin, engine, call.head)
                    .map_err(LabeledError::from)
            }
        }

        #[cfg(test)]
        mod $test {
            use super::*;
            use crate::test::test_polars_plugin_command;
            use nu_protocol::ShellError;

            #[test]
            fn test_examples() -> Result<(), ShellError> {
                test_polars_plugin_command(&$command)
            }
        }
    };

    ($command: ident, $name: expr, $desc: expr, $examples: expr, $func: ident, $test: ident, $ddot: expr) => {
        #[derive(Clone)]
        pub struct $command;

        impl PluginCommand for $command {
            type Plugin = PolarsPlugin;

            fn signature(&self) -> Signature {
                Signature::build($name)
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
                let lazy = NuLazyFrame::try_from_pipeline_coerce(plugin, input, call.head)
                    .map_err(LabeledError::from)?;
                let lazy = NuLazyFrame::new(lazy.from_eager, lazy.into_polars().$func($ddot));
                lazy.to_pipeline_data(plugin, engine, call.head)
                    .map_err(LabeledError::from)
            }
        }

        #[cfg(test)]
        mod $test {
            use super::*;
            use crate::test::test_polars_plugin_command;
            use nu_protocol::ShellError;

            #[test]
            fn test_examples() -> Result<(), ShellError> {
                test_polars_plugin_command(&$command)
            }
        }
    };

    ($command: ident, $name: expr, $desc: expr, $examples: expr, $func: ident?, $test: ident) => {
        #[derive(Clone)]
        pub struct $command;

        impl PluginCommand for $command {
            type Plugin = PolarsPlugin;

            fn name(&self) -> &str {
                $name
            }

            fn usage(&self) -> &str {
                $desc
            }
            fn signature(&self) -> Signature {
                Signature::build(self.name())
                    .input_output_type(
                        Type::Custom("dataframe".into()),
                        Type::Custom("dataframe".into()),
                    )
                    .category(Category::Custom("lazyframe".into()))
            }

            fn examples(&self) -> Vec<Example> {
                $examples
            }

            fn run(
                &self,
                plugin: &Self::Plugin,
                engine: &EngineInterface,
                call: &EvaluatedCall,
                input: PipelineData,
            ) -> Result<PipelineData, LabeledError> {
                let lazy = NuLazyFrame::try_from_pipeline_coerce(plugin, input, call.head)
                    .map_err(LabeledError::from)?;

                let lazy = NuLazyFrame::new(
                    lazy.from_eager,
                    lazy.to_polars()
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

                lazy.to_pipeline_data(plugin, engine, call.head)
                    .map_err(LabeledError::from)
            }
        }

        #[cfg(test)]
        mod $test {
            use super::*;
            use crate::test::test_polars_plugin_command;
            use nu_protocol::ShellError;

            #[test]
            fn test_examples() -> Result<(), ShellError> {
                test_polars_plugin_command(&$command)
            }
        }
    };
}

// LazyReverse command
// Expands to a command definition for reverse
lazy_command!(
    LazyReverse,
    "polars reverse",
    "Reverses the LazyFrame",
    vec![Example {
        description: "Reverses the dataframe.",
        example: "[[a b]; [6 2] [4 2] [2 2]] | polars into-df | polars reverse",
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
    vec![Example {
        description: "Caches the result into a new LazyFrame",
        example: "[[a b]; [6 2] [4 2] [2 2]] | polars into-df | polars reverse | polars cache",
        result: None,
    }],
    cache,
    test_cache
);
