use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{dataframe::PolarsData, Signature, TaggedDictBuilder, UntaggedValue};

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
        let values = args
            .context
            .scope
            .get_vars()
            .into_iter()
            .filter_map(|(name, value)| {
                if let UntaggedValue::DataFrame(PolarsData::EagerDataFrame(df)) = &value.value {
                    let mut data = TaggedDictBuilder::new(value.tag.clone());

                    let rows = df.as_ref().height();
                    let cols = df.as_ref().width();

                    data.insert_value("name", name.as_ref());
                    data.insert_value("rows", format!("{}", rows));
                    data.insert_value("columns", format!("{}", cols));

                    match value.tag.anchor {
                        Some(AnchorLocation::File(name)) => data.insert_value("location", name),
                        Some(AnchorLocation::Url(name)) => data.insert_value("location", name),
                        Some(AnchorLocation::Source(text)) => {
                            let loc_name = text.slice(0..text.end);
                            data.insert_value("location", loc_name.text)
                        }
                        None => data.insert_value("location", "stream"),
                    }

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
            example: "dataframe list",
            result: None,
        }]
    }
}
