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
        "from-csv"
    }

    fn signature(&self) -> Signature {
        Signature::build("from-csv")
            .named(
                "separator",
                SyntaxShape::String,
                "a character to separate columns, defaults to ','",
            )
            .switch("headerless", "don't treat the first row as column names")
    }

    fn usage(&self) -> &str {
        "Parse text as .csv and create table."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, from_csv)?.run()
    }
}

fn from_csv(
    FromCSVArgs {
        headerless,
        separator,
    }: FromCSVArgs,
    runnable_context: RunnableContext,
) -> Result<OutputStream, ShellError> {
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

    from_delimited_data(headerless, sep, "CSV", runnable_context)
}
