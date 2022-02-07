use crate::{commands::dataframe::utils::parse_polars_error, prelude::*};
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{Column, NuDataFrame},
    Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::Tagged;

use super::utils::convert_columns;

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe melt"
    }

    fn usage(&self) -> &str {
        "[DataFrame] Unpivot a DataFrame from wide to long format"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe melt")
            .required_named(
                "columns",
                SyntaxShape::Table,
                "column names for melting",
                Some('c'),
            )
            .required_named(
                "values",
                SyntaxShape::Table,
                "column names used as value columns",
                Some('v'),
            )
            .named(
                "variable_name",
                SyntaxShape::String,
                "optional name for variable column",
                Some('r'),
            )
            .named(
                "value_name",
                SyntaxShape::String,
                "optional name for value column",
                Some('l'),
            )
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "melt dataframe",
            example:
                "[[a b c d]; [x 1 4 a] [y 2 5 b] [z 3 6 c]] | dataframe to-df | dataframe melt -c [b c] -v [a d]",
            result: Some(vec![NuDataFrame::try_from_columns(
                vec![
                    Column::new(
                        "b".to_string(),
                        vec![
                            UntaggedValue::int(1).into(),
                            UntaggedValue::int(2).into(),
                            UntaggedValue::int(3).into(),
                            UntaggedValue::int(1).into(),
                            UntaggedValue::int(2).into(),
                            UntaggedValue::int(3).into(),
                        ],
                    ),
                    Column::new(
                        "c".to_string(),
                        vec![
                            UntaggedValue::int(4).into(),
                            UntaggedValue::int(5).into(),
                            UntaggedValue::int(6).into(),
                            UntaggedValue::int(4).into(),
                            UntaggedValue::int(5).into(),
                            UntaggedValue::int(6).into(),
                        ],
                    ),
                    Column::new(
                        "variable".to_string(),
                        vec![
                            UntaggedValue::string("a").into(),
                            UntaggedValue::string("a").into(),
                            UntaggedValue::string("a").into(),
                            UntaggedValue::string("d").into(),
                            UntaggedValue::string("d").into(),
                            UntaggedValue::string("d").into(),
                        ],
                    ),
                    Column::new(
                        "value".to_string(),
                        vec![
                            UntaggedValue::string("x").into(),
                            UntaggedValue::string("y").into(),
                            UntaggedValue::string("z").into(),
                            UntaggedValue::string("a").into(),
                            UntaggedValue::string("b").into(),
                            UntaggedValue::string("c").into(),
                        ],
                    ),
                ],
                &Span::default(),
            )
            .expect("simple df for test should not fail")
            .into_value(Tag::default())]),
        }]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();

    let id_col: Vec<Value> = args.req_named("columns")?;
    let val_col: Vec<Value> = args.req_named("values")?;

    let value_name: Option<Tagged<String>> = args.get_flag("value_name")?;
    let variable_name: Option<Tagged<String>> = args.get_flag("variable_name")?;

    let (id_col_string, id_col_span) = convert_columns(&id_col, &tag)?;
    let (val_col_string, val_col_span) = convert_columns(&val_col, &tag)?;

    let (df, _) = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

    check_column_datatypes(df.as_ref(), &id_col_string, &id_col_span)?;
    check_column_datatypes(df.as_ref(), &val_col_string, &val_col_span)?;

    let mut res = df
        .as_ref()
        .melt(&id_col_string, &val_col_string)
        .map_err(|e| parse_polars_error::<&str>(&e, &tag.span, None))?;

    if let Some(name) = &variable_name {
        res.rename("variable", &name.item)
            .map_err(|e| parse_polars_error::<&str>(&e, &name.tag.span, None))?;
    }

    if let Some(name) = &value_name {
        res.rename("value", &name.item)
            .map_err(|e| parse_polars_error::<&str>(&e, &name.tag.span, None))?;
    }

    Ok(OutputStream::one(NuDataFrame::dataframe_to_value(res, tag)))
}

fn check_column_datatypes<T: AsRef<str>>(
    df: &polars::prelude::DataFrame,
    cols: &[T],
    col_span: &Span,
) -> Result<(), ShellError> {
    if cols.is_empty() {
        return Err(ShellError::labeled_error(
            "Merge error",
            "empty column list",
            col_span,
        ));
    }

    // Checking if they are same type
    if cols.len() > 1 {
        for w in cols.windows(2) {
            let l_series = df
                .column(w[0].as_ref())
                .map_err(|e| parse_polars_error::<&str>(&e, col_span, None))?;

            let r_series = df
                .column(w[1].as_ref())
                .map_err(|e| parse_polars_error::<&str>(&e, col_span, None))?;

            if l_series.dtype() != r_series.dtype() {
                return Err(ShellError::labeled_error_with_secondary(
                    "Merge error",
                    "found different column types in list",
                    col_span,
                    format!(
                        "datatypes {} and {} are incompatible",
                        l_series.dtype(),
                        r_series.dtype()
                    ),
                    col_span,
                ));
            }
        }
    }

    Ok(())
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
