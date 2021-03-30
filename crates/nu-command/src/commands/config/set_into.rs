use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    ConfigPath, Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::Tagged;

pub struct SubCommand;

#[derive(Deserialize)]
pub struct Arguments {
    set_into: Tagged<String>,
}

#[async_trait]
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

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        set_into(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Store the contents of the pipeline as a path",
            example: "echo ['/usr/bin' '/bin'] | config set_into path",
            result: None,
        }]
    }
}

pub async fn set_into(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name = args.call_info.name_tag.clone();
    let ctx = EvaluationContext::from_args(&args);
    let scope = args.scope.clone();
    let (Arguments { set_into: v }, input) = args.process().await?;

    let path = match scope.get_var("config-path") {
        Some(Value {
            value: UntaggedValue::Primitive(Primitive::FilePath(path)),
            ..
        }) => Some(path),
        _ => nu_data::config::default_path().ok(),
    };

    let mut result = nu_data::config::read(&name, &path)?;

    let rows: Vec<Value> = input.collect().await;
    let key = v.to_string();

    Ok(if rows.is_empty() {
        return Err(ShellError::labeled_error(
            "No values given for set_into",
            "needs value(s) from pipeline",
            v.tag(),
        ));
    } else if rows.len() == 1 {
        // A single value
        let value = &rows[0];

        result.insert(key, value.clone());

        config::write(&result, &path)?;
        ctx.reload_config(&ConfigPath::Global(
            path.expect("Global config path is always some"),
        ))
        .await?;

        OutputStream::one(ReturnSuccess::value(
            UntaggedValue::Row(result.into()).into_value(name),
        ))
    } else {
        // Take in the pipeline as a table
        let value = UntaggedValue::Table(rows).into_value(name.clone());

        result.insert(key, value);

        config::write(&result, &path)?;
        ctx.reload_config(&ConfigPath::Global(
            path.expect("Global config path is always some"),
        ))
        .await?;

        OutputStream::one(ReturnSuccess::value(
            UntaggedValue::Row(result.into()).into_value(name),
        ))
    })
}
