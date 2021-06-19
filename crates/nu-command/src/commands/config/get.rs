use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, UntaggedValue, Value};

pub struct SubCommand;

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
    let ctx = &args.context;

    let column_path = args.req(0)?;

    let result = if let Some(global_cfg) = &ctx.configs().lock().global_config {
        let result = UntaggedValue::row(global_cfg.vars.clone()).into_value(&name);
        let value = crate::commands::filters::get::get_column_path(&column_path, &result)?;
        Ok(match value {
            Value {
                value: UntaggedValue::Table(list),
                ..
            } => OutputStream::from_stream(list.into_iter()),
            x => OutputStream::one(x),
        })
    } else {
        let value = UntaggedValue::Error(crate::commands::config::err_no_global_cfg_present())
            .into_value(name);

        Ok(OutputStream::one(value))
    };

    result
}
