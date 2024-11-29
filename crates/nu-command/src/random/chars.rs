use nu_engine::command_prelude::*;
use nu_protocol::format_filesize_from_conf;
use rand::{
    distributions::{Alphanumeric, Distribution},
    thread_rng,
};

const DEFAULT_CHARS_LENGTH: usize = 25;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "random chars"
    }

    fn signature(&self) -> Signature {
        Signature::build("random chars")
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .allow_variants_without_examples(true)
            .named(
                "length",
                SyntaxShape::OneOf(vec![SyntaxShape::Int, SyntaxShape::Filesize]),
                "Number of chars (default 25)",
                Some('l'),
            )
            .category(Category::Random)
    }

    fn description(&self) -> &str {
        "Generate random chars uniformly distributed over ASCII letters and numbers: a-z, A-Z and 0-9."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["generate", "character", "symbol", "alphanumeric"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        chars(engine_state, stack, call)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Generate a string with 25 random chars",
                example: "random chars",
                result: None,
            },
            Example {
                description: "Generate random chars with specified length",
                example: "random chars --length 20",
                result: None,
            },
            Example {
                description: "Generate one kilobyte of random chars",
                example: "random chars --length 1kb",
                result: None,
            },
        ]
    }
}

fn chars(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let length: Option<Value> = call.get_flag(engine_state, stack, "length")?;
    let length = if let Some(length_val) = length {
        match length_val {
            Value::Int { val, .. } => usize::try_from(val).map_err(|_| ShellError::InvalidValue {
                valid: "a non-negative int or filesize".into(),
                actual: val.to_string(),
                span: length_val.span(),
            }),
            Value::Filesize { val, .. } => {
                usize::try_from(val).map_err(|_| ShellError::InvalidValue {
                    valid: "a non-negative int or filesize".into(),
                    actual: format_filesize_from_conf(val, engine_state.get_config()),
                    span: length_val.span(),
                })
            }
            val => Err(ShellError::RuntimeTypeMismatch {
                expected: Type::custom("int or filesize"),
                actual: val.get_type(),
                span: val.span(),
            }),
        }?
    } else {
        DEFAULT_CHARS_LENGTH
    };

    let mut rng = thread_rng();

    let random_string = Alphanumeric
        .sample_iter(&mut rng)
        .take(length)
        .map(char::from)
        .collect::<String>();

    Ok(PipelineData::Value(
        Value::string(random_string, span),
        None,
    ))
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
