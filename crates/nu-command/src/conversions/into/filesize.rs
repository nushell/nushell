use nu_cmd_base::input_handler::{operate, CellPathOnlyArgs};
use nu_engine::command_prelude::*;

use nu_utils::get_system_locale;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "into filesize"
    }

    fn signature(&self) -> Signature {
        Signature::build("into filesize")
            .input_output_types(vec![
                (Type::Int, Type::Filesize),
                (Type::Number, Type::Filesize),
                (Type::String, Type::Filesize),
                (Type::Filesize, Type::Filesize),
                (Type::Table(vec![]), Type::Table(vec![])),
                (Type::Record(vec![]), Type::Record(vec![])),
                (
                    Type::List(Box::new(Type::Int)),
                    Type::List(Box::new(Type::Filesize)),
                ),
                (
                    Type::List(Box::new(Type::Number)),
                    Type::List(Box::new(Type::Filesize)),
                ),
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::Filesize)),
                ),
                (
                    Type::List(Box::new(Type::Filesize)),
                    Type::List(Box::new(Type::Filesize)),
                ),
                // Catch all for heterogeneous lists.
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Filesize)),
                ),
            ])
            .allow_variants_without_examples(true)
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, convert data at the given cell paths.",
            )
            .category(Category::Conversions)
    }

    fn usage(&self) -> &str {
        "Convert value to filesize."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "number", "bytes"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
        let args = CellPathOnlyArgs::from(cell_paths);
        operate(action, args, input, call.head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Convert string to filesize in table",
                example: r#"[[device size]; ["/dev/sda1" "200"] ["/dev/loop0" "50"]] | into filesize size"#,
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "device" => Value::test_string("/dev/sda1"),
                        "size" =>   Value::test_filesize(200),
                    }),
                    Value::test_record(record! {
                        "device" => Value::test_string("/dev/loop0"),
                        "size" =>   Value::test_filesize(50),
                    }),
                ])),
            },
            Example {
                description: "Convert string to filesize",
                example: "'2' | into filesize",
                result: Some(Value::test_filesize(2)),
            },
            Example {
                description: "Convert float to filesize",
                example: "8.3 | into filesize",
                result: Some(Value::test_filesize(8)),
            },
            Example {
                description: "Convert int to filesize",
                example: "5 | into filesize",
                result: Some(Value::test_filesize(5)),
            },
            Example {
                description: "Convert file size to filesize",
                example: "4KB | into filesize",
                result: Some(Value::test_filesize(4000)),
            },
            Example {
                description: "Convert string with unit to filesize",
                example: "'-1KB' | into filesize",
                result: Some(Value::test_filesize(-1000)),
            },
        ]
    }
}

pub fn action(input: &Value, _args: &CellPathOnlyArgs, span: Span) -> Value {
    let value_span = input.span();
    match input {
        Value::Filesize { .. } => input.clone(),
        Value::Int { val, .. } => Value::filesize(*val, value_span),
        Value::Float { val, .. } => Value::filesize(*val as i64, value_span),
        Value::String { val, .. } => match int_from_string(val, value_span) {
            Ok(val) => Value::filesize(val, value_span),
            Err(error) => Value::error(error, value_span),
        },
        Value::Nothing { .. } => Value::filesize(0, value_span),
        other => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "string and int".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: span,
                src_span: value_span,
            },
            span,
        ),
    }
}
fn int_from_string(a_string: &str, span: Span) -> Result<i64, ShellError> {
    // Get the Locale so we know what the thousands separator is
    let locale = get_system_locale();

    // Now that we know the locale, get the thousands separator and remove it
    // so strings like 1,123,456 can be parsed as 1123456
    let no_comma_string = a_string.replace(locale.separator(), "");
    let clean_string = no_comma_string.trim();

    // Hadle negative file size
    if let Some(stripped_string) = clean_string.strip_prefix('-') {
        match stripped_string.parse::<bytesize::ByteSize>() {
            Ok(n) => Ok(-(n.as_u64() as i64)),
            Err(_) => Err(ShellError::CantConvert {
                to_type: "int".into(),
                from_type: "string".into(),
                span,
                help: None,
            }),
        }
    } else {
        match clean_string.parse::<bytesize::ByteSize>() {
            Ok(n) => Ok(n.0 as i64),
            Err(_) => Err(ShellError::CantConvert {
                to_type: "int".into(),
                from_type: "string".into(),
                span,
                help: None,
            }),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
