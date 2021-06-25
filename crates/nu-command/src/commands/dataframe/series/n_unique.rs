use crate::{commands::dataframe::utils::parse_polars_error, prelude::*};
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::NuSeries, Primitive, Signature, TaggedDictBuilder, UntaggedValue, Value,
};

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe count-unique"
    }

    fn usage(&self) -> &str {
        "[Series] Counts unique value"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe count-unique")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Counts unique values",
            example: "[1 1 2 2 3 3 4] | dataframe to-series | dataframe count-unique",
            result: None,
        }]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();

    let series = NuSeries::try_from_stream(&mut args.input, &tag.span)?;

    let res = series
        .as_ref()
        .n_unique()
        .map_err(|e| parse_polars_error::<&str>(&e, &tag.span, None))?;

    let value = Value {
        value: UntaggedValue::Primitive(Primitive::Int(res as i64)),
        tag: tag.clone(),
    };

    let mut data = TaggedDictBuilder::new(tag);
    data.insert_value("count-unique", value);

    Ok(OutputStream::one(data.into_value()))
}
