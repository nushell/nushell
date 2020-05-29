use crate::commands::to_delimited_data::to_delimited_data;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Primitive, Signature, SyntaxShape, UntaggedValue, Value};

pub struct ToCSV;

#[derive(Deserialize)]
pub struct ToCSVArgs {
    headerless: bool,
    separator: Option<Value>,
}

#[async_trait]
impl WholeStreamCommand for ToCSV {
    fn name(&self) -> &str {
        "to csv"
    }

    fn signature(&self) -> Signature {
        Signature::build("to csv")
            .named(
                "separator",
                SyntaxShape::String,
                "a character to separate columns, defaults to ','",
                Some('s'),
            )
            .switch(
                "headerless",
                "do not output the columns names as the first row",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Convert table into .csv text "
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        to_csv(args, registry)
    }
}

fn to_csv(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let stream = async_stream! {
        let name = args.call_info.name_tag.clone();
        let (ToCSVArgs { separator, headerless }, mut input) = args.process(&registry).await?;
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

        let mut result = to_delimited_data(headerless, sep, "CSV", input, name)?;

        while let Some(item) = result.next().await {
            yield item;
        }
    };

    Ok(stream.to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::ToCSV;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(ToCSV {})
    }
}
