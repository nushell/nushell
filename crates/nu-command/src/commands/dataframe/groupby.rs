use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{NuDataFrame, NuGroupBy, PolarsData},
    Signature, SyntaxShape, UntaggedValue, Value,
};

use super::utils::convert_columns;

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "pls groupby"
    }

    fn usage(&self) -> &str {
        "Creates a groupby object that can be used for other aggregations"
    }

    fn signature(&self) -> Signature {
        Signature::build("pls groupby").required(
            "by columns",
            SyntaxShape::Table,
            "groupby columns",
        )
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        groupby(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Grouping by column a",
            example: "echo [[a b]; [one 1] [one 2]] | pls convert | pls groupby [a]",
            result: None,
        }]
    }
}

fn groupby(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let mut args = args.evaluate_once()?;

    // Extracting the names of the columns to perform the groupby
    let by_columns: Vec<Value> = args.req(0)?;
    let (columns_string, col_span) = convert_columns(&by_columns, &tag)?;

    // The operation is only done in one dataframe. Only one input is
    // expected from the InputStream
    match args.input.next() {
        None => Err(ShellError::labeled_error(
            "No input received",
            "missing dataframe input from stream",
            &tag,
        )),
        Some(value) => {
            if let UntaggedValue::DataFrame(PolarsData::EagerDataFrame(nu_df)) = value.value {
                let df = match nu_df.dataframe {
                    Some(df) => df,
                    None => unreachable!("No dataframe in nu_dataframe"),
                };

                // This is the expensive part of the groupby; to create the
                // groups that will be used for grouping the data in the
                // dataframe. Once it has been done these values can be stored
                // in the NuGroupBy
                let groupby = df.groupby(&columns_string).map_err(|e| {
                    ShellError::labeled_error("Groupby error", format!("{}", e), col_span)
                })?;

                let groups = groupby.get_groups().to_vec();
                let groupby = Value {
                    tag: value.tag,
                    value: UntaggedValue::DataFrame(PolarsData::GroupBy(NuGroupBy::new(
                        NuDataFrame::new_with_name(df, nu_df.name),
                        columns_string,
                        groups,
                    ))),
                };

                Ok(OutputStream::one(groupby))
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
