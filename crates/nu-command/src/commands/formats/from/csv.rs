use super::delimited::from_delimited_data;
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Primitive, Signature, SyntaxShape, UntaggedValue, Value};

pub struct FromCsv;

impl WholeStreamCommand for FromCsv {
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
                "noheaders",
                "don't treat the first row as column names",
                Some('n'),
            )
    }

    fn usage(&self) -> &str {
        "Parse text as .csv and create table."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        from_csv(args)
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
                example: "open data.txt | from csv --noheaders",
                result: None,
            },
            Example {
                description: "Convert comma-separated data to a table, ignoring headers",
                example: "open data.txt | from csv -n",
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

fn from_csv(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name = args.call_info.name_tag.clone();

    let noheaders = args.has_flag("noheaders");
    let separator: Option<Value> = args.get_flag("separator")?;
    let input = args.input;

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

    from_delimited_data(noheaders, sep, "CSV", input, name)
}

#[cfg(test)]
mod tests {
    use super::FromCsv;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(FromCsv {})
    }
}
