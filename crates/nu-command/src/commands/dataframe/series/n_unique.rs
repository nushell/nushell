use crate::{commands::dataframe::utils::parse_polars_error, prelude::*};
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{Column, NuDataFrame},
    Primitive, Signature, UntaggedValue, Value,
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
            example: "[1 1 2 2 3 3 4] | dataframe to-df | dataframe count-unique",
            result: Some(vec![NuDataFrame::try_from_columns(
                vec![Column::new(
                    "count_unique".to_string(),
                    vec![UntaggedValue::int(4).into()],
                )],
                &Span::default(),
            )
            .expect("simple df for test should not fail")
            .into_value(Tag::default())]),
        }]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();

    let (df, df_tag) = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

    let res = df
        .as_series(&df_tag.span)?
        .n_unique()
        .map_err(|e| parse_polars_error::<&str>(&e, &tag.span, None))?;

    let value = Value {
        value: UntaggedValue::Primitive(Primitive::Int(res as i64)),
        tag: tag.clone(),
    };

    let df = NuDataFrame::try_from_columns(
        vec![Column::new("count_unique".to_string(), vec![value])],
        &tag.span,
    )?;

    Ok(OutputStream::one(df.into_value(tag)))
}

#[cfg(test)]
mod tests {
    use super::DataFrame;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test_dataframe as test_examples;

        test_examples(DataFrame {})
    }
}
