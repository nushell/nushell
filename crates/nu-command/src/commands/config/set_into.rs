use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;

pub struct SubCommand;

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "config set_into"
    }

    fn signature(&self) -> Signature {
        Signature::build("config set_into").required(
            "set_into",
            SyntaxShape::String,
            "sets a variable from values in the pipeline",
        )
    }

    fn usage(&self) -> &str {
        "Sets a value in the config"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        set_into(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Store the contents of the pipeline as a path",
            example: "echo ['/usr/bin' '/bin'] | config set_into path",
            result: None,
        }]
    }
}

pub fn set_into(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name = args.call_info.name_tag.clone();
    let ctx = &args.context;

    let set_into: Tagged<String> = args.req(0)?;

    let rows: Vec<Value> = args.input.collect();
    let key = set_into.to_string();

    let result = if let Some(global_cfg) = &mut ctx.configs().lock().global_config {
        if rows.is_empty() {
            return Err(ShellError::labeled_error(
                "No values given for set_into",
                "needs value(s) from pipeline",
                set_into.tag(),
            ));
        } else if rows.len() == 1 {
            // A single value
            let value = &rows[0];

            global_cfg.vars.insert(key, value.clone());
        } else {
            // Take in the pipeline as a table
            let value = UntaggedValue::Table(rows).into_value(name.clone());

            global_cfg.vars.insert(key, value);
        }

        global_cfg.write()?;
        ctx.reload_config(global_cfg)?;

        let value = UntaggedValue::row(global_cfg.vars.clone()).into_value(name);

        Ok(OutputStream::one(value))
    } else {
        let value = UntaggedValue::Error(crate::commands::config::err_no_global_cfg_present())
            .into_value(name);

        Ok(OutputStream::one(value))
    };

    result
}
