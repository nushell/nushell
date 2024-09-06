/// Definition of multiple Expression commands using a macro rule
/// All of these expressions have an identical body and only require
/// to have a change in the name, description and expression function

#[macro_export]
macro_rules! lazy_command {
    ($command: ident, $name: expr, $desc: expr, $examples: expr, $func: ident, $test: ident) => {
        use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
        use nu_protocol::{Category, Example, LabeledError, PipelineData, Signature, Type};
        /// Definition of multiple lazyframe commands using a macro rule
        /// All of these commands have an identical body and only require
        /// to have a change in the name, description and function
        use $crate::dataframe::values::NuLazyFrame;
        use $crate::values::CustomValueSupport;
        use $crate::PolarsPlugin;

        #[derive(Clone)]
        pub struct $command;

        impl PluginCommand for $command {
            type Plugin = PolarsPlugin;

            fn name(&self) -> &str {
                $name
            }

            fn description(&self) -> &str {
                $desc
            }

            fn signature(&self) -> Signature {
                Signature::build(self.name())
                    .description($desc)
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
            use nu_protocol::ShellError;
            use $crate::test::test_polars_plugin_command;

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
                    .description($desc)
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
            use nu_protocol::ShellError;
            use $crate::test::test_polars_plugin_command;

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

            fn description(&self) -> &str {
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
            use nu_protocol::ShellError;
            use $crate::test::test_polars_plugin_command;

            #[test]
            fn test_examples() -> Result<(), ShellError> {
                test_polars_plugin_command(&$command)
            }
        }
    };
}

// The structs defined in this file are structs that form part of other commands
// since they share a similar name
#[macro_export]
macro_rules! expr_command {
    ($command: ident, $name: expr, $desc: expr, $examples: expr, $func: ident, $test: ident) => {
        #[derive(Clone)]
        pub struct $command;

        impl PluginCommand for $command {
            type Plugin = PolarsPlugin;

            fn name(&self) -> &str {
                $name
            }

            fn description(&self) -> &str {
                $desc
            }

            fn signature(&self) -> Signature {
                Signature::build(self.name())
                    .description($desc)
                    .input_output_type(
                        Type::Custom("expression".into()),
                        Type::Custom("expression".into()),
                    )
                    .category(Category::Custom("expression".into()))
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
                let expr = NuExpression::try_from_pipeline(plugin, input, call.head)
                    .map_err(LabeledError::from)?;
                let expr: NuExpression = expr.into_polars().$func().into();
                expr.to_pipeline_data(plugin, engine, call.head)
                    .map_err(LabeledError::from)
            }
        }

        #[cfg(test)]
        mod $test {
            use super::*;
            use nu_protocol::ShellError;
            use $crate::test::test_polars_plugin_command;

            #[test]
            fn test_examples() -> Result<(), ShellError> {
                test_polars_plugin_command(&$command)
            }
        }
    };

    ($command: ident, $name: expr, $desc: expr, $examples: expr, $func: ident, $test: ident, $ddof: expr) => {
        #[derive(Clone)]
        pub struct $command;

        impl PluginCommand for $command {
            type Plugin = PolarsPlugin;

            fn signature(&self) -> Signature {
                Signature::build(self.name())
                    .description($desc)
                    .input_output_type(
                        Type::Custom("expression".into()),
                        Type::Custom("expression".into()),
                    )
                    .category(Category::Custom("expression".into()))
                    .plugin_examples($examples)
            }

            fn run(
                &self,
                _plugin: &Self::Plugin,
                engine: &EngineInterface,
                call: &EvaluatedCall,
                input: PipelineData,
            ) -> Result<PipelineData, LabeledError> {
                let expr = NuExpression::try_from_pipeline(input, call.head)
                    .map_err(LabeledError::from)?;
                let expr: NuExpression = expr.into_polars().$func($ddof).into();
                expr.to_pipeline_data(plugin, engine, call.head)
                    .map_err(LabeledError::from)
            }
        }

        #[cfg(test)]
        mod $test {
            use super::*;
            use $crate::test::test_polars_plugin_command;

            #[test]
            fn test_examples() -> Result<(), ShellError> {
                test_polars_plugin_command(&$command)
            }
        }
    };
}

// The structs defined in this file are structs that form part of other commands
// since they share a similar name
#[macro_export]
macro_rules! lazy_expr_command {
    ($command: ident, $name: expr, $desc: expr, $examples: expr, $func: ident, $test: ident) => {
        #[derive(Clone)]
        pub struct $command;

        impl PluginCommand for $command {
            type Plugin = PolarsPlugin;

            fn name(&self) -> &str {
                $name
            }

            fn description(&self) -> &str {
                $desc
            }

            fn signature(&self) -> Signature {
                Signature::build(self.name())
                    .description($desc)
                    .input_output_types(vec![
                        (
                            Type::Custom("expression".into()),
                            Type::Custom("expression".into()),
                        ),
                        (
                            Type::Custom("dataframe".into()),
                            Type::Custom("dataframe".into()),
                        ),
                    ])
                    .category(Category::Custom("expression".into()))
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
                let value = input.into_value(call.head)?;
                if NuDataFrame::can_downcast(&value) || NuLazyFrame::can_downcast(&value) {
                    let lazy = NuLazyFrame::try_from_value_coerce(plugin, &value)
                        .map_err(LabeledError::from)?;
                    let lazy = NuLazyFrame::new(lazy.from_eager, lazy.to_polars().$func());
                    lazy.to_pipeline_data(plugin, engine, call.head)
                        .map_err(LabeledError::from)
                } else {
                    let expr =
                        NuExpression::try_from_value(plugin, &value).map_err(LabeledError::from)?;
                    let expr: NuExpression = expr.into_polars().$func().into();
                    expr.to_pipeline_data(plugin, engine, call.head)
                        .map_err(LabeledError::from)
                }
            }
        }

        #[cfg(test)]
        mod $test {
            use super::*;
            use nu_protocol::ShellError;
            use $crate::test::test_polars_plugin_command;

            #[test]
            fn test_examples() -> Result<(), ShellError> {
                test_polars_plugin_command(&$command)
            }
        }
    };

    ($command: ident, $name: expr, $desc: expr, $examples: expr, $func: ident, $test: ident, $ddof: expr) => {
        #[derive(Clone)]
        pub struct $command;

        impl PluginCommand for $command {
            type Plugin = PolarsPlugin;

            fn name(&self) -> &str {
                $name
            }

            fn description(&self) -> &str {
                $desc
            }
            fn signature(&self) -> Signature {
                Signature::build(self.name())
                    .input_output_types(vec![
                        (
                            Type::Custom("expression".into()),
                            Type::Custom("expression".into()),
                        ),
                        (
                            Type::Custom("dataframe".into()),
                            Type::Custom("dataframe".into()),
                        ),
                    ])
                    .category(Category::Custom("expression".into()))
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
                let value = input.into_value(call.head)?;
                if NuDataFrame::can_downcast(&value) || NuLazyFrame::can_downcast(&value) {
                    let lazy = NuLazyFrame::try_from_value_coerce(plugin, &value)
                        .map_err(LabeledError::from)?;
                    let lazy = NuLazyFrame::new(lazy.from_eager, lazy.to_polars().$func($ddof));
                    lazy.to_pipeline_data(plugin, engine, call.head)
                        .map_err(LabeledError::from)
                } else {
                    let expr = NuExpression::try_from_value(plugin, &value)?;
                    let expr: NuExpression = expr.into_polars().$func($ddof).into();
                    expr.to_pipeline_data(plugin, engine, call.head)
                        .map_err(LabeledError::from)
                }
            }
        }

        #[cfg(test)]
        mod $test {
            use super::*;
            use nu_protocol::ShellError;
            use $crate::test::test_polars_plugin_command;

            #[test]
            fn test_examples() -> Result<(), ShellError> {
                test_polars_plugin_command(&$command)
            }
        }
    };
}
