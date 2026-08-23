use nu_plugin::PluginCommand;
use nu_protocol::{
    Example, LabeledError, ShellError, Signature, SyntaxShape, Type, Value,
    shell_error::generic::GenericError,
};
use polars::{
    prelude::{Expr, date_ranges},
    time::{ClosedWindow, Duration},
};

use crate::{
    PolarsPlugin,
    values::{CustomValueSupport, NuExpression, PolarsPluginType},
};

pub struct DateRange;

impl PluginCommand for DateRange {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars date-range"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build(self.name())
            .named(
                "start",
                SyntaxShape::Any,
                "Start of the date range expression. This supports a string, date, or datetime expression.",
                Some('s'),
            )
            .named(
                "end",
                SyntaxShape::Any,
                "End of the date range expression. This supports a string, date, or datetime expression.",
                Some('e'),
            )
            .named(
                "interval",
                SyntaxShape::Any,
                "Interval of the date range expression. This supports a string or duration expression.",
                Some('i'),
            )
            .named(
                "closed",
                SyntaxShape::String,
                "Closed window of the date range expression. This supports 'left', 'right', 'both', or 'none'.",
                Some('c'),
            )
            .input_output_types(vec![(Type::Any, PolarsPluginType::NuExpression.into())])
            .category(nu_protocol::Category::Custom("dataframe".into()))
    }

    fn description(&self) -> &str {
        "Creates a date range expression."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &nu_plugin::EngineInterface,
        call: &nu_plugin::EvaluatedCall,
        mut input: nu_protocol::PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::LabeledError> {
        let metadata = input.take_metadata();
        command(plugin, engine, call)
            .map_err(LabeledError::from)
            .map(|pd| pd.set_metadata(metadata))
    }
}

fn command(
    plugin: &PolarsPlugin,
    engine: &nu_plugin::EngineInterface,
    call: &nu_plugin::EvaluatedCall,
) -> Result<nu_protocol::PipelineData, ShellError> {
    let start: Option<Expr> = call
        .get_flag_value("start")
        .map(|ref v| NuExpression::try_from_value(plugin, v))
        .transpose()?
        .map(|e| e.into_polars());

    let end: Option<Expr> = call
        .get_flag_value("end")
        .map(|ref v| NuExpression::try_from_value(plugin, v))
        .transpose()?
        .map(|e| e.into_polars());

    let interval: Option<Duration> = call
        .get_flag_value("interval")
        .map(|ref v| value_to_duration(v))
        .transpose()?;

    let closed_window: ClosedWindow = call
        .get_flag_value("closed")
        .map(|ref v| {
            let closed_str = v.as_str()?;
            match closed_str {
                "left" => Ok(ClosedWindow::Left),
                "right" => Ok(ClosedWindow::Right),
                "both" => Ok(ClosedWindow::Both),
                "none" => Ok(ClosedWindow::None),
                _ => Err(ShellError::Generic(GenericError::new(
                    "Invalid closed window",
                    format!("Invalid closed window: {}", closed_str),
                    v.span(),
                ))),
            }
        })
        .transpose()?
        .unwrap_or(ClosedWindow::Both);

    let range = date_ranges(
        start,
        end,
        interval,
        None, // todo - num samples is not yet implemented in polars
        closed_window,
    )
    .map_err(|e| {
        ShellError::Generic(
            GenericError::new(
                "Failed to create date range",
                format!("Failed to create date range: {}", e),
                call.head,
            )
            .with_source(e),
        )
    })?;

    NuExpression::from(range).to_pipeline_data(plugin, engine, call.head)
}

fn value_to_duration(value: &Value) -> Result<polars::prelude::Duration, ShellError> {
    match value {
        Value::Duration { val, .. } => Ok(Duration::new(*val)),
        Value::String { val, .. } => Ok(polars::prelude::Duration::try_parse(&val.to_string())
            .map_err(|e| {
                ShellError::Generic(GenericError::new(
                    "Failed to parse duration",
                    format!("Failed to parse duration: {}", e),
                    value.span(),
                ))
            })?),
        _ => Err(ShellError::Generic(GenericError::new(
            "Invalid duration",
            format!("Expected a string for duration: received {value:?}"),
            value.span(),
        ))),
    }
}
