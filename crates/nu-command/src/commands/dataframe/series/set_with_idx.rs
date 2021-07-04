use crate::{commands::dataframe::utils::parse_polars_error, prelude::*};
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{dataframe::NuSeries, Primitive, Signature, SyntaxShape, UntaggedValue, Value};
use polars::prelude::{ChunkSet, DataType, IntoSeries};

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe set-with-idx"
    }

    fn usage(&self) -> &str {
        "[Series] Sets value in the given index"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe set-with-idx")
            .required("value", SyntaxShape::Any, "value to be inserted in series")
            .required_named(
                "indices",
                SyntaxShape::Any,
                "list of indices indicating where to set the value",
                Some('i'),
            )
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Set value in selected rows from series",
            example: r#"let series = ([4 1 5 2 4 3] | dataframe to-series);
    let indices = ([0 2] | dataframe to-series);
    $series | dataframe set-with-idx 6 -i $indices"#,
            result: None,
        }]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let value: Value = args.req(0)?;
    let indices: Value = args.req_named("indices")?;

    let indices = match &indices.value {
        UntaggedValue::DataFrame(nu_protocol::dataframe::PolarsData::Series(series)) => Ok(series),
        _ => Err(ShellError::labeled_error(
            "Incorrect type",
            "can only use a series for set command",
            value.tag.span,
        )),
    }?;

    let casted = match indices.as_ref().dtype() {
        DataType::UInt32 | DataType::UInt64 | DataType::Int32 | DataType::Int64 => indices
            .as_ref()
            .cast_with_dtype(&DataType::UInt32)
            .map_err(|e| parse_polars_error::<&str>(&e, &value.tag.span, None)),
        _ => Err(ShellError::labeled_error_with_secondary(
            "Incorrect type",
            "Series with incorrect type",
            &value.tag.span,
            "Consider using a Series with type int type",
            &value.tag.span,
        )),
    }?;

    let indices = casted
        .u32()
        .map_err(|e| parse_polars_error::<&str>(&e, &value.tag.span, None))?
        .into_iter()
        .filter_map(|val| val.map(|v| v as usize));

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
                .set_at_idx(indices, Some(*val))
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
                .set_at_idx(
                    indices,
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
                .set_at_idx(indices, Some(val.as_ref()))
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
