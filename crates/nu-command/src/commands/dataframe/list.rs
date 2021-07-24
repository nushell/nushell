use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{Column, NuDataFrame},
    Signature, UntaggedValue, Value,
};

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
        let data = args
            .context
            .scope
            .get_vars()
            .into_iter()
            .filter_map(|(name, value)| {
                if let UntaggedValue::DataFrame(df) = &value.value {
                    let rows = Value {
                        value: (df.as_ref().height() as i64).into(),
                        tag: Tag::default(),
                    };

                    let cols = Value {
                        value: (df.as_ref().width() as i64).into(),
                        tag: Tag::default(),
                    };

                    let location = match value.tag.anchor {
                        Some(AnchorLocation::File(name)) => name,
                        Some(AnchorLocation::Url(name)) => name,
                        Some(AnchorLocation::Source(text)) => text.slice(0..text.end).text,
                        None => "stream".to_string(),
                    };

                    let location = Value {
                        value: location.into(),
                        tag: Tag::default(),
                    };

                    let name = Value {
                        value: name.into(),
                        tag: Tag::default(),
                    };

                    Some((name, rows, cols, location))
                } else {
                    None
                }
            });

        let mut name = Column::new_empty("name".to_string());
        let mut rows = Column::new_empty("rows".to_string());
        let mut cols = Column::new_empty("columns".to_string());
        let mut location = Column::new_empty("location".to_string());

        for tuple in data {
            name.push(tuple.0);
            rows.push(tuple.1);
            cols.push(tuple.2);
            location.push(tuple.3);
        }

        let tag = args.call_info.name_tag;
        let df = NuDataFrame::try_from_columns(vec![name, rows, cols, location], &tag.span)?;
        Ok(OutputStream::one(df.into_value(tag)))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Lists loaded dataframes in current scope",
            example: "let a = ([[a b];[1 2] [3 4]] | dataframe to-df); dataframe list",
            result: Some(vec![NuDataFrame::try_from_columns(
                vec![
                    Column::new("name".to_string(), vec![UntaggedValue::string("$a").into()]),
                    Column::new("rows".to_string(), vec![UntaggedValue::int(2).into()]),
                    Column::new("columns".to_string(), vec![UntaggedValue::int(2).into()]),
                    Column::new(
                        "location".to_string(),
                        vec![UntaggedValue::string("stream").into()],
                    ),
                ],
                &Span::default(),
            )
            .expect("simple df for test should not fail")
            .into_value(Tag::default())]),
        }]
    }
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
