use crate::commands::WholeStreamCommand;
use crate::data::value;
use crate::prelude::*;
use crate::utils::data_processing::map_max;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use num_traits::cast::ToPrimitive;

pub struct MapMaxBy;

#[derive(Deserialize)]
pub struct MapMaxByArgs {
    column_name: Option<Tagged<String>>,
}

impl WholeStreamCommand for MapMaxBy {
    fn name(&self) -> &str {
        "map-max-by"
    }

    fn signature(&self) -> Signature {
        Signature::build("map-max-by").named(
            "column_name",
            SyntaxShape::String,
            "the name of the column to map-max the table's rows",
        )
    }

    fn usage(&self) -> &str {
        "Creates a new table with the data from the tables rows maxed by the column given."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, map_max_by)?.run()
    }
}

pub fn map_max_by(
    MapMaxByArgs { column_name }: MapMaxByArgs,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let stream = async_stream! {
        let values: Vec<Value> = input.values.collect().await;


        if values.is_empty() {
            yield Err(ShellError::labeled_error(
                    "Expected table from pipeline",
                    "requires a table input",
                    name
                ))
        } else {

            let map_by_column = if let Some(column_to_map) = column_name {
                Some(column_to_map.item().clone())
            } else {
                None
            };

            match map_max(&values[0], map_by_column, name) {
                Ok(table_maxed) => yield ReturnSuccess::value(table_maxed),
                Err(err) => yield Err(err)
            }
        }
    };

    Ok(stream.to_output_stream())
}
