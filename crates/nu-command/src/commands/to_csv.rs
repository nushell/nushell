use crate::commands::to_delimited_data::to_delimited_data;
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Primitive, Signature, SyntaxShape, UntaggedValue, Value};

pub struct ToCSV;

#[derive(Deserialize)]
pub struct ToCSVArgs {
    noheaders: bool,
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
                "noheaders",
                "do not output the columns names as the first row",
                Some('n'),
            )
    }

    fn usage(&self) -> &str {
        "Convert table into .csv text "
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        to_csv(args).await
    }
}

async fn to_csv(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name = args.call_info.name_tag.clone();
    let (
        ToCSVArgs {
            separator,
            noheaders,
        },
        input,
    ) = args.process().await?;
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
                    return Err(ShellError::labeled_error(
                        "Expected a single separator char from --separator",
                        "requires a single character string input",
                        tag,
                    ));
                };
                vec_s[0]
            }
        }
        _ => ',',
    };

    to_delimited_data(noheaders, sep, "CSV", input, name).await
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::ToCSV;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(ToCSV {})
    }
}
