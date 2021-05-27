use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{NuDataFrame, PolarsData},
    Signature, TaggedDictBuilder, UntaggedValue,
};

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "pls list"
    }

    fn usage(&self) -> &str {
        "Lists stored dataframes"
    }

    fn signature(&self) -> Signature {
        Signature::build("pls list")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let args = args.evaluate_once()?;

        let values = args
            .context
            .scope
            .get_vars()
            .into_iter()
            .filter_map(|(name, value)| {
                if let UntaggedValue::DataFrame(PolarsData::EagerDataFrame(NuDataFrame {
                    dataframe: Some(df),
                    name: file_name,
                })) = &value.value
                {
                    let mut data = TaggedDictBuilder::new(value.tag.clone());

                    let rows = df.height();
                    let cols = df.width();

                    data.insert_value("name", name.as_ref());
                    data.insert_value("file", file_name.as_ref());
                    data.insert_value("rows", format!("{}", rows));
                    data.insert_value("columns", format!("{}", cols));

                    Some(data.into_value())
                } else {
                    None
                }
            });

        Ok(OutputStream::from_stream(values))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Lists loaded dataframes in current scope",
            example: "pls list",
            result: None,
        }]
    }
}
