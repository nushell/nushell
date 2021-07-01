use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::NuSeries, Primitive, Signature, TaggedDictBuilder, UntaggedValue, Value,
};

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe count-null"
    }

    fn usage(&self) -> &str {
        "[Series] Counts null values"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe count-null")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Counts null values",
            example: r#"let s = ([1 1 0 0 3 3 4] | dataframe to-series);
    ($s / ss) | dataframe count-null"#,
            result: None,
        }]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();

    let series = NuSeries::try_from_stream(&mut args.input, &tag.span)?;

    let res = series.as_ref().null_count();

    let value = Value {
        value: UntaggedValue::Primitive(Primitive::Int(res as i64)),
        tag: tag.clone(),
    };

    let mut data = TaggedDictBuilder::new(tag);
    data.insert_value("count-null", value);

    Ok(OutputStream::one(data.into_value()))
}
