use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, UntaggedValue, Value};

pub struct SubCommand;

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "config set"
    }

    fn signature(&self) -> Signature {
        Signature::build("config set")
            .required("key", SyntaxShape::ColumnPath, "variable name to set")
            .required("value", SyntaxShape::Any, "value to use")
    }

    fn usage(&self) -> &str {
        "Sets a value in the config"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        set(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Set auto pivoting",
                example: "config set pivot_mode always",
                result: None,
            },
            Example {
                description: "Set line editor options",
                example: "config set line_editor [[edit_mode, completion_type]; [emacs circular]]",
                result: None,
            },
            Example {
                description: "Set coloring options",
                example: "config set color_config [[header_align header_bold]; [left $true]]",
                result: None,
            },
            Example {
                description: "Set nested options",
                example: "config set color_config.header_color white",
                result: None,
            },
        ]
    }
}

pub fn set(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name = args.call_info.name_tag.clone();
    let ctx = &args.context;

    let column_path = args.req(0)?;
    let mut value: Value = args.req(1)?;

    let result = if let Some(global_cfg) = &mut ctx.configs().lock().global_config {
        let configuration = UntaggedValue::row(global_cfg.vars.clone()).into_value(&name);

        if let UntaggedValue::Table(rows) = &value.value {
            if rows.len() == 1 && rows[0].is_row() {
                value = rows[0].clone();
            }
        }

        match configuration.forgiving_insert_data_at_column_path(&column_path, value) {
            Ok(Value {
                value: UntaggedValue::Row(changes),
                ..
            }) => {
                global_cfg.vars = changes.entries;
                global_cfg.write()?;
                ctx.reload_config(global_cfg)?;

                let value = UntaggedValue::row(global_cfg.vars.clone()).into_value(name);
                Ok(OutputStream::one(value))
            }
            Ok(_) => Ok(OutputStream::empty()),
            Err(reason) => Err(reason),
        }
    } else {
        let value = UntaggedValue::Error(crate::commands::config::err_no_global_cfg_present())
            .into_value(name);

        Ok(OutputStream::one(value))
    };

    result
}
