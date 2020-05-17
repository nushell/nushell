use crate::commands::from_delimited_data::from_delimited_data;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Primitive, Signature, SyntaxShape, UntaggedValue, Value};

pub struct FromCSV;

#[derive(Deserialize)]
pub struct FromCSVArgs {
    headerless: bool,
    separator: Option<Value>,
}

impl WholeStreamCommand for FromCSV {
    fn name(&self) -> &str {
        "from csv"
    }

    fn signature(&self) -> Signature {
        Signature::build("from csv")
            .named(
                "separator",
                SyntaxShape::String,
                "a character to separate columns, defaults to ','",
                Some('s'),
            )
            .switch(
                "headerless",
                "don't treat the first row as column names",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Parse text as .csv and create table."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        from_csv(args, registry)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Convert comma-separated data to a table",
                example: "open data.txt | from csv",
                result: None,
            },
            Example {
                description: "Convert comma-separated data to a table, ignoring headers",
                example: "open data.txt | from csv --headerless",
                result: None,
            },
            Example {
                description: "Convert semicolon-separated data to a table",
                example: "open data.txt | from csv --separator ';'",
                result: None,
            },
        ]
    }
}

fn from_csv(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let name = args.call_info.name_tag.clone();
    let stream = async_stream! {
        let (FromCSVArgs { headerless, separator }, mut input) = args.process(&registry).await?;
        let sep = match separator {
            Some(Value {
                value: UntaggedValue::Primitive(Primitive::String(s)),
                tag,
                ..
            }) => {
                if s == r"\t" {
                    '\t'
                } else {
                    let vec_s: Vec<char> = s.chars().collect();
                    if vec_s.len() != 1 {
                        yield Err(ShellError::labeled_error(
                            "Expected a single separator char from --separator",
                            "requires a single character string input",
                            tag,
                        ));
                        return;
                    };
                    vec_s[0]
                }
            }
            _ => ',',
        };

        let mut result = from_delimited_data(headerless, sep, "CSV", input, name)?;
        while let Some(item) = result.next().await {
            yield item;
        }

    };

    Ok(stream.to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::FromCSV;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(FromCSV {})
    }
}
