use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::NuSeries, Primitive, Signature, TaggedDictBuilder, UntaggedValue, Value,
};

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe arg-max"
    }

    fn usage(&self) -> &str {
        "[Series] Return index for max value in series"
    }

    fn extra_usage(&self) -> &str {
        ""
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe arg-max")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Returns index for max value",
            example: "[1 3 2] | dataframe to-series | dataframe arg-max",
            result: None,
        }]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();

    let series = NuSeries::try_from_stream(&mut args.input, &tag.span)?;

    let res = series.as_ref().arg_max();

    let value = match res {
        Some(index) => UntaggedValue::Primitive(Primitive::Int(index as i64)),
        None => UntaggedValue::Primitive(Primitive::Nothing),
    };

    let value = Value {
        value,
        tag: tag.clone(),
    };

    let mut data = TaggedDictBuilder::new(tag);
    data.insert_value("arg-max", value);

    Ok(OutputStream::one(data.into_value()))
}
