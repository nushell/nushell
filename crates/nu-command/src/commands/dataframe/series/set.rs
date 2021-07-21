use crate::{commands::dataframe::utils::parse_polars_error, prelude::*};
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::NuDataFrame, Primitive, Signature, SyntaxShape, UntaggedValue, Value,
};
use polars::prelude::{ChunkSet, DataType, IntoSeries};

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe set"
    }

    fn usage(&self) -> &str {
        "[Series] Sets value where given mask is true"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe set")
            .required("value", SyntaxShape::Any, "value to be inserted in series")
            .required_named(
                "mask",
                SyntaxShape::Any,
                "mask indicating insertions",
                Some('m'),
            )
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Shifts the values by a given period",
            example: r#"let s = ([1 2 2 3 3] | dataframe to-df | dataframe shift 2);
    let mask = ($s | dataframe is-null);
    $s | dataframe set 0 --mask $mask"#,
            result: None,
        }]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let value: Value = args.req(0)?;
    let mask: Value = args.req_named("mask")?;

    let mask_df = match &mask.value {
        UntaggedValue::DataFrame(df) => Ok(df),
        _ => Err(ShellError::labeled_error(
            "Incorrect type",
            "can only use a series as mask",
            value.tag.span,
        )),
    }?;

    let mask_series = mask_df.as_series(&mask.tag.span)?;

    let bool_mask = match mask_series.dtype() {
        DataType::Boolean => mask_series
            .bool()
            .map_err(|e| parse_polars_error::<&str>(&e, &mask.tag.span, None)),
        _ => Err(ShellError::labeled_error(
            "Incorrect type",
            "can only use bool series as mask",
            value.tag.span,
        )),
    }?;

    let (df, df_tag) = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;
    let series = df.as_series(&df_tag.span)?;

    match &value.value {
        UntaggedValue::Primitive(Primitive::Int(val)) => {
            let chunked = series.i64().map_err(|e| {
                parse_polars_error::<&str>(
                    &e,
                    &value.tag.span,
                    Some("The value has to match the set value type"),
                )
            })?;

            let res = chunked
                .set(bool_mask, Some(*val))
                .map_err(|e| parse_polars_error::<&str>(&e, &value.tag.span, None))?;

            let df = NuDataFrame::try_from_series(vec![res.into_series()], &tag.span)?;
            Ok(OutputStream::one(df.into_value(df_tag)))
        }
        UntaggedValue::Primitive(Primitive::Decimal(val)) => {
            let chunked = series.as_ref().f64().map_err(|e| {
                parse_polars_error::<&str>(
                    &e,
                    &value.tag.span,
                    Some("The value has to match the series type"),
                )
            })?;

            let res = chunked
                .set(
                    bool_mask,
                    Some(
                        val.to_f64()
                            .expect("internal error: expected f64-compatible decimal"),
                    ),
                )
                .map_err(|e| parse_polars_error::<&str>(&e, &value.tag.span, None))?;

            let df = NuDataFrame::try_from_series(vec![res.into_series()], &tag.span)?;
            Ok(OutputStream::one(df.into_value(df_tag)))
        }
        UntaggedValue::Primitive(Primitive::String(val)) => {
            let chunked = series.as_ref().utf8().map_err(|e| {
                parse_polars_error::<&str>(
                    &e,
                    &value.tag.span,
                    Some("The value has to match the series type"),
                )
            })?;

            let res = chunked
                .set(bool_mask, Some(val.as_ref()))
                .map_err(|e| parse_polars_error::<&str>(&e, &value.tag.span, None))?;

            let mut res = res.into_series();
            res.rename("string");

            let df = NuDataFrame::try_from_series(vec![res.into_series()], &tag.span)?;
            Ok(OutputStream::one(df.into_value(df_tag)))
        }
        _ => Err(ShellError::labeled_error(
            "Incorrect type",
            format!(
                "this value cannot be set in a series of type '{}'",
                series.dtype()
            ),
            value.tag.span,
        )),
    }
}
