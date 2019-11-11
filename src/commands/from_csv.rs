use crate::commands::from_structured_data::from_structured_data;
use crate::commands::WholeStreamCommand;
use crate::data::{Primitive, Value};
use crate::prelude::*;

pub struct FromCSV;

#[derive(Deserialize)]
pub struct FromCSVArgs {
    headerless: bool,
    separator: Option<Tagged<Value>>,
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
        Some(Tagged {
            item: Value::Primitive(Primitive::String(s)),
            tag,
            ..
        }) => {
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
        _ => ',',
    };

    from_structured_data(headerless, sep, "CSV", runnable_context)
}
