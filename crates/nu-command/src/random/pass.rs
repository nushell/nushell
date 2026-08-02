use aegis_password_generator::types::PasswordConfig;
use nu_engine::command_prelude::*;
use nu_protocol::shell_error::generic::GenericError;

const DEFAULT_PASSWORD_LENGTH: usize = 12;

#[derive(Clone)]
pub struct RandomPass;

impl Command for RandomPass {
    fn name(&self) -> &str {
        "random pass"
    }

    fn signature(&self) -> Signature {
        Signature::build("random pass")
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .allow_variants_without_examples(true)
            .named(
                "chars",
                SyntaxShape::Int,
                "Length of the generated password (default 12).",
                Some('c'),
            )
            .switch("no-uppercase", "Exclude uppercase letters A-Z.", Some('u'))
            .switch("no-lowercase", "Exclude lowercase letters a-z.", Some('l'))
            .switch("no-numbers", "Exclude numbers 0-9.", Some('n'))
            .switch("no-symbols", "Exclude symbols like !@#$%.", Some('s'))
            .switch(
                "include-ambiguous",
                "Include ambiguous characters O, 0, l, 1.",
                None,
            )
            .switch(
                "include-similar",
                "Include similar characters i, l, 1.",
                None,
            )
            .switch(
                "require-each-type",
                "Guarantee at least one char from each enabled character type.",
                None,
            )
            .category(Category::Random)
    }

    fn description(&self) -> &str {
        "Generate a cryptologically secure password."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["password", "generate", "crypto", "secure"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;

        let chars: Option<i64> = call.get_flag(engine_state, stack, "chars")?;
        let no_uppercase = call.has_flag(engine_state, stack, "no-uppercase")?;
        let no_lowercase = call.has_flag(engine_state, stack, "no-lowercase")?;
        let no_numbers = call.has_flag(engine_state, stack, "no-numbers")?;
        let no_symbols = call.has_flag(engine_state, stack, "no-symbols")?;
        let include_ambiguous = call.has_flag(engine_state, stack, "include-ambiguous")?;
        let include_similar = call.has_flag(engine_state, stack, "include-similar")?;
        let require_each_type = call.has_flag(engine_state, stack, "require-each-type")?;

        let length = chars.map(|c| c as usize).unwrap_or(DEFAULT_PASSWORD_LENGTH);

        let config = PasswordConfig::default()
            .with_length(length)
            .with_uppercase(!no_uppercase)
            .with_lowercase(!no_lowercase)
            .with_numbers(!no_numbers)
            .with_symbols(!no_symbols)
            .with_exclude_ambiguous(!include_ambiguous)
            .with_exclude_similar(!include_similar)
            .with_require_each_type(require_each_type);

        match config.generate() {
            Ok(password) => Ok(Value::string(password, span).into_pipeline_data()),
            Err(e) => Err(ShellError::Generic(GenericError::new(
                "Password generation failed",
                e.to_string(),
                span,
            ))),
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Generate a 12-character password with defaults",
                example: "random pass",
                result: None,
            },
            Example {
                description: "Generate a 20-character password",
                example: "random pass --chars 20",
                result: None,
            },
            Example {
                description: "Generate a password without symbols",
                example: "random pass --no-symbols",
                result: None,
            },
            Example {
                description: "Generate a password with only uppercase letters and numbers",
                example: "random pass --no-lowercase --no-symbols",
                result: None,
            },
            Example {
                description: "Generate a password including ambiguous characters and requiring each type",
                example: "random pass --include-ambiguous --require-each-type",
                result: None,
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(RandomPass)
    }
}
