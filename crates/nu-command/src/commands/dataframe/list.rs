use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, TaggedDictBuilder, UntaggedValue, Value};

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe list"
    }

    fn usage(&self) -> &str {
        "Lists stored dataframes"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe list")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let args = args.evaluate_once()?;

        let mut dataframes: Vec<Value> = Vec::new();
        for (name, value) in args.context.scope.get_vars() {
            if let UntaggedValue::DataFrame(df) = value.value {
                let mut data = TaggedDictBuilder::new(value.tag);

                let polars_df = df.dataframe.unwrap();

                let rows = polars_df.height();
                let cols = polars_df.width();

                data.insert_value("name", name);
                data.insert_value("file", df.name);
                data.insert_value("rows", format!("{}", rows));
                data.insert_value("columns", format!("{}", cols));

                dataframes.push(data.into_value());
            }
        }

        Ok(OutputStream::from_stream(dataframes.into_iter()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Lists loaded dataframes in current scope",
            example: "dataframe list",
            result: None,
        }]
    }
}
