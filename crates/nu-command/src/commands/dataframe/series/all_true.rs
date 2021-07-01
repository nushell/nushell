use crate::{commands::dataframe::utils::parse_polars_error, prelude::*};
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{dataframe::NuSeries, Signature, TaggedDictBuilder, UntaggedValue, Value};

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe all-true"
    }

    fn usage(&self) -> &str {
        "[Series] Returns true if all values are true"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe all-true")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Returns true if all values are true",
                example: "[$true $true $true] | dataframe to-series | dataframe all-true",
                result: None,
            },
            Example {
                description: "Checks the result from a comparison",
                example: r#"let s = ([5 6 2 8] | dataframe to-series);
    let res = ($s > 9);
    $res | dataframe all-true"#,
                result: None,
            },
        ]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();

    let series = NuSeries::try_from_stream(&mut args.input, &tag.span)?;

    let bool = series.as_ref().bool().map_err(|e| {
        parse_polars_error::<&str>(
            &e,
            &tag.span,
            Some("all-true only works with series of type bool"),
        )
    })?;

    let res = bool.all_true();

    let value = Value {
        value: UntaggedValue::Primitive(res.into()),
        tag: tag.clone(),
    };

    let mut data = TaggedDictBuilder::new(tag);
    data.insert_value("all_true", value);

    Ok(OutputStream::one(data.into_value()))
}
