use crate::{commands::dataframe::utils::parse_polars_error, prelude::*};
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{dataframe::NuSeries, Primitive, Signature, SyntaxShape, UntaggedValue, Value};
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
            example: r#"let s = ([1 2 2 3 3] | dataframe to-series | dataframe shift 2);
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

    let bool_mask = match &mask.value {
        UntaggedValue::DataFrame(nu_protocol::dataframe::PolarsData::Series(series)) => {
            match series.as_ref().dtype() {
                DataType::Boolean => series
                    .as_ref()
                    .bool()
                    .map_err(|e| parse_polars_error::<&str>(&e, &mask.tag.span, None)),
                _ => Err(ShellError::labeled_error(
                    "Incorrect type",
                    "can only use bool series as mask",
                    value.tag.span,
                )),
            }
        }
        _ => Err(ShellError::labeled_error(
            "Incorrect type",
            "can only use bool series as mask",
            value.tag.span,
        )),
    }?;

    let series = NuSeries::try_from_stream(&mut args.input, &tag.span)?;

    match &value.value {
        UntaggedValue::Primitive(Primitive::Int(val)) => {
            let chunked = series.as_ref().i64().map_err(|e| {
                parse_polars_error::<&str>(
                    &e,
                    &value.tag.span,
                    Some("The value has to match the set value type"),
                )
            })?;

            let res = chunked
                .set(bool_mask, Some(*val))
                .map_err(|e| parse_polars_error::<&str>(&e, &value.tag.span, None))?;

            Ok(OutputStream::one(NuSeries::series_to_value(
                res.into_series(),
                tag,
            )))
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

            Ok(OutputStream::one(NuSeries::series_to_value(
                res.into_series(),
                tag,
            )))
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

            Ok(OutputStream::one(NuSeries::series_to_value(
                res.into_series(),
                tag,
            )))
        }
        _ => Err(ShellError::labeled_error(
            "Incorrect type",
            format!(
                "this value cannot be set in a series of type '{}'",
                series.as_ref().dtype()
            ),
            value.tag.span,
        )),
    }
}
