use nu_engine::command_prelude::*;
use nu_protocol::shell_error::generic::GenericError;
use rand::seq::{IndexedRandom, SliceRandom};

const DEFAULT_PASSWORD_LENGTH: usize = 12;
const MAX_PASSWORD_LENGTH: usize = 128;

const UPPERCASE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const LOWERCASE: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
const NUMBERS: &[u8] = b"0123456789";
const SYMBOLS: &[u8] = b"!@#$%^&*()_+-=[]{}|;':\",.<>/?`~";

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
        let require_each_type = call.has_flag(engine_state, stack, "require-each-type")?;
        // `--include-ambiguous` and `--include-similar` are still parsed and type-checked, but
        // they don't change the character sets, as in earlier releases.
        let _ = call.has_flag(engine_state, stack, "include-ambiguous")?;
        let _ = call.has_flag(engine_state, stack, "include-similar")?;

        let length = chars.map(|c| c as usize).unwrap_or(DEFAULT_PASSWORD_LENGTH);
        let sets: Vec<&[u8]> = [
            (!no_uppercase, UPPERCASE),
            (!no_lowercase, LOWERCASE),
            (!no_numbers, NUMBERS),
            (!no_symbols, SYMBOLS),
        ]
        .into_iter()
        .filter_map(|(enabled, set)| enabled.then_some(set))
        .collect();

        let error = |msg: &'static str| {
            ShellError::Generic(GenericError::new("Password generation failed", msg, span))
        };
        if length == 0 {
            return Err(error("Password length must be greater than 0"));
        }
        if length > MAX_PASSWORD_LENGTH {
            return Err(error("Password length must be less than or equal to 128"));
        }
        if require_each_type && sets.len() < 4 {
            return Err(error(
                "Invalid password configuration: You've enabled `require_each_type`, but have \
                 also excluded one or more character sets.",
            ));
        }
        if sets.is_empty() {
            return Err(error(
                "Invalid password configuration: At least one character type must be included.",
            ));
        }

        Ok(Value::string(generate_password(length, &sets), span).into_pipeline_data())
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

/// Builds a password from `sets`, each of which must be non-empty.
///
/// The password starts with one character from every set, so it is longer than `length` when
/// `length` is less than the number of sets. The rest comes from all sets combined, and the
/// result is shuffled.
fn generate_password(length: usize, sets: &[&[u8]]) -> String {
    let mut rng = rand::rng();
    let mut password: Vec<u8> = sets
        .iter()
        .filter_map(|set| set.choose(&mut rng).copied())
        .collect();
    let all: Vec<u8> = sets.concat();
    for _ in password.len()..length {
        password.extend(all.choose(&mut rng));
    }
    password.shuffle(&mut rng);
    password.into_iter().map(char::from).collect()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(RandomPass)
    }
}
