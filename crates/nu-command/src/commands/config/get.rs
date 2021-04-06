use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    ColumnPath, Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};

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
    let scope = args.scope.clone();
    let (Arguments { column_path }, _) = args.process()?;

    let path = match scope.get_var("config-path") {
        Some(Value {
            value: UntaggedValue::Primitive(Primitive::FilePath(path)),
            ..
        }) => Some(path),
        _ => nu_data::config::default_path().ok(),
    };

    let result = UntaggedValue::row(nu_data::config::read(&name, &path)?).into_value(&name);

    let value = crate::commands::get::get_column_path(&column_path, &result)?;

    Ok(match value {
        Value {
            value: UntaggedValue::Table(list),
            ..
        } => {
            let list: Vec<_> = list
                .iter()
                .map(|x| ReturnSuccess::value(x.clone()))
                .collect();

            list.into_iter().to_output_stream()
        }
        x => OutputStream::one(ReturnSuccess::value(x)),
    })
}
