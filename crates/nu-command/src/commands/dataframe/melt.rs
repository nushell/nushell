use crate::{commands::dataframe::utils::parse_polars_error, prelude::*};
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{NuDataFrame, PolarsData},
    Signature, SyntaxShape, UntaggedValue, Value,
};

use super::utils::convert_columns;

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "pls melt"
    }

    fn usage(&self) -> &str {
        "Unpivot a DataFrame from wide to long format"
    }

    fn signature(&self) -> Signature {
        Signature::build("pls join")
            .required("id_columns", SyntaxShape::Table, "Id columns for melting")
            .required(
                "value_columns",
                SyntaxShape::Table,
                "columns used as value columns",
            )
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "melt dataframe",
            example: "echo [[a b]; [a 2] [b 4] [a 6]] | pls convert | pls melt [a] [b]",
            result: None,
        }]
    }
}

fn command(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let mut args = args.evaluate_once()?;

    let id_col: Vec<Value> = args.req(0)?;
    let val_col: Vec<Value> = args.req(1)?;

    let (id_col_string, id_col_span) = convert_columns(&id_col, &tag)?;
    let (val_col_string, val_col_span) = convert_columns(&val_col, &tag)?;

    match args.input.next() {
        None => Err(ShellError::labeled_error(
            "No input received",
            "missing dataframe input from stream",
            &tag,
        )),
        Some(value) => {
            if let UntaggedValue::DataFrame(PolarsData::EagerDataFrame(df)) = value.value {
                check_column_datatypes(df.as_ref(), &id_col_string, &id_col_span)?;
                check_column_datatypes(df.as_ref(), &val_col_string, &val_col_span)?;

                let res = df
                    .as_ref()
                    .melt(&id_col_string, &val_col_string)
                    .map_err(|e| parse_polars_error::<&str>(&e, &tag.span, None))?;

                let value = Value {
                    value: UntaggedValue::DataFrame(PolarsData::EagerDataFrame(NuDataFrame::new(
                        res,
                    ))),
                    tag: tag.clone(),
                };

                Ok(OutputStream::one(value))
            } else {
                Err(ShellError::labeled_error(
                    "No dataframe in stream",
                    "no dataframe found in input stream",
                    &tag,
                ))
            }
        }
    }
}

fn check_column_datatypes<T: AsRef<str>>(
    df: &polars::prelude::DataFrame,
    cols: &[T],
    col_span: &Span,
) -> Result<(), ShellError> {
    if cols.len() == 0 {
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
                .map_err(|e| parse_polars_error::<&str>(&e, &col_span, None))?;

            let r_series = df
                .column(w[1].as_ref())
                .map_err(|e| parse_polars_error::<&str>(&e, &col_span, None))?;

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
