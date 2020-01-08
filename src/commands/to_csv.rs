use crate::commands::to_delimited_data::to_delimited_data;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Primitive, Signature, UntaggedValue, Value};

pub struct ToCSV;

#[derive(Deserialize)]
pub struct ToCSVArgs {
    headerless: bool,
    separator: Option<Value>,
}

impl WholeStreamCommand for ToCSV {
    fn name(&self) -> &str {
        "to-csv"
    }

    fn signature(&self) -> Signature {
        Signature::build("to-csv").switch(
            "headerless",
            "do not output the columns names as the first row",
        )
    }

    fn usage(&self) -> &str {
        "Convert table into .csv text "
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, to_csv)?.run()
    }
}

fn to_csv(
    ToCSVArgs {
        separator,
        headerless,
    }: ToCSVArgs,
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

    to_delimited_data(headerless, sep, "CSV", runnable_context)
}
