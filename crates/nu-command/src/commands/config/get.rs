use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};

pub struct SubCommand;

#[derive(Deserialize)]
pub struct Arguments {
    column_path: ColumnPath,
}

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "config get"
    }

    fn signature(&self) -> Signature {
        Signature::build("config get").required(
            "get",
            SyntaxShape::ColumnPath,
            "value to get from the config",
        )
    }

    fn usage(&self) -> &str {
        "Gets a value from the config"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        get(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Get the current startup commands",
            example: "config get startup",
            result: None,
        }]
    }
}

pub fn get(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name = args.call_info.name_tag.clone();
    let ctx = EvaluationContext::from_args(&args);

    let (Arguments { column_path }, _) = args.process().await?;

    let result = if let Some(global_cfg) = &ctx.configs.lock().global_config {
        let result = UntaggedValue::row(global_cfg.vars.clone()).into_value(&name);
        let value = crate::commands::get::get_column_path(&column_path, &result)?;
        Ok(match value {
            Value {
                value: UntaggedValue::Table(list),
                ..
            } => list.into_iter().to_output_stream(),
            x => OutputStream::one(ReturnSuccess::value(x)),
        })
    } else {
        Ok(vec![ReturnSuccess::value(UntaggedValue::Error(
            crate::commands::config::err_no_global_cfg_present(),
        ))]
        .into_iter()
        .to_output_stream())
    };

    result
}
