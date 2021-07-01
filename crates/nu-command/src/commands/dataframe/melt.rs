use crate::{commands::dataframe::utils::parse_polars_error, prelude::*};
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{dataframe::NuDataFrame, Signature, SyntaxShape, Value};

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
            .required("id_columns", SyntaxShape::Table, "Id columns for melting")
            .rest(SyntaxShape::Any, "columns used as value columns")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "melt dataframe",
            example: "[[a b]; [a 2] [b 4] [a 6]] | dataframe to-df | dataframe melt a b",
            result: None,
        }]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();

    let id_col: Vec<Value> = args.req(0)?;
    let val_col: Vec<Value> = args.rest(1)?;

    let (id_col_string, id_col_span) = convert_columns(&id_col, &tag)?;
    let (val_col_string, val_col_span) = convert_columns(&val_col, &tag)?;

    let df = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

    check_column_datatypes(df.as_ref(), &id_col_string, &id_col_span)?;
    check_column_datatypes(df.as_ref(), &val_col_string, &val_col_span)?;

    let res = df
        .as_ref()
        .melt(&id_col_string, &val_col_string)
        .map_err(|e| parse_polars_error::<&str>(&e, &tag.span, None))?;

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
