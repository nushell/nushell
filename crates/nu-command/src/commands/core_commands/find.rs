use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Dictionary, Signature, SyntaxShape, UntaggedValue, Value};

pub struct Find;

impl WholeStreamCommand for Find {
    fn name(&self) -> &str {
        "find"
    }

    fn signature(&self) -> Signature {
        Signature::build("find").rest("rest", SyntaxShape::String, "search term")
    }

    fn usage(&self) -> &str {
        "Find text in the output of a previous command"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        find(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Search pipeline output for multiple terms",
                example: r#"ls | find toml md sh"#,
                result: None,
            },
            Example {
                description: "Search strings for term(s)",
                example: r#"echo Cargo.toml | find toml"#,
                result: Some(vec![Value::from("Cargo.toml")]),
            },
            Example {
                description: "Search a number list for term(s)",
                example: r#"[1 2 3 4 5] | find 5"#,
                result: Some(vec![UntaggedValue::int(5).into()]),
            },
            Example {
                description: "Search string list for term(s)",
                example: r#"[moe larry curly] | find l"#,
                result: Some(vec![Value::from("larry"), Value::from("curly")]),
            },
        ]
    }
}

fn row_contains(row: &Dictionary, search_terms: Vec<String>) -> bool {
    for term in search_terms {
        for (k, v) in &row.entries {
            let key = k.to_string().trim().to_lowercase();
            let value = v.convert_to_string().trim().to_lowercase();
            if key.contains(&term) || value.contains(&term) {
                return true;
            }
        }
    }

    false
}

fn find(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let rest: Vec<Value> = args.rest(0)?;

    Ok(args
        .input
        .filter(move |row| match &row.value {
            UntaggedValue::Row(row) => {
                let sterms: Vec<String> = rest
                    .iter()
                    .map(|t| t.convert_to_string().trim().to_lowercase())
                    .collect();
                row_contains(row, sterms)
            }
            UntaggedValue::Primitive(_p) => {
                // eprint!("prim {}", p.type_name());
                let sterms: Vec<String> = rest
                    .iter()
                    .map(|t| t.convert_to_string().trim().to_lowercase())
                    .collect();

                let prim_string = &row.convert_to_string().trim().to_lowercase();
                for term in sterms {
                    if prim_string.contains(&term) {
                        return true;
                    }
                }

                false
            }
            _ => false,
        })
        .into_output_stream())
}

#[cfg(test)]
mod tests {
    use super::Find;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Find {})
    }
}
