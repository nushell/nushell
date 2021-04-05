use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    ColumnPath, ConfigPath, Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};

pub struct SubCommand;

#[derive(Deserialize)]
pub struct Arguments {
    column_path: ColumnPath,
    value: Value,
}

#[async_trait]
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
    let ctx = EvaluationContext::from_args(&args);
    let scope = args.scope.clone();
    let (
        Arguments {
            column_path,
            mut value,
        },
        _,
    ) = args.process()?;

    let path = match scope.get_var("config-path") {
        Some(Value {
            value: UntaggedValue::Primitive(Primitive::FilePath(path)),
            ..
        }) => Some(path),
        _ => nu_data::config::default_path().ok(),
    };

    let raw_entries = nu_data::config::read(&name, &path)?;
    let configuration = UntaggedValue::row(raw_entries).into_value(&name);

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
            config::write(&changes.entries, &path)?;
            ctx.reload_config(&ConfigPath::Global(
                path.expect("Global config path is always some"),
            ))
            ?;

            Ok(OutputStream::one(ReturnSuccess::value(
                UntaggedValue::Row(changes).into_value(name),
            )))
        }
        Ok(_) => Ok(OutputStream::empty()),
        Err(reason) => Err(reason),
    }
}
