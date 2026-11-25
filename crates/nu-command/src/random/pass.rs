use nu_engine::command_prelude::*;
use nu_protocol::shell_error::generic::GenericError;
use rand::seq::{IndexedRandom, SliceRandom};

const DEFAULT_PASSWORD_LENGTH: usize = 12;
const MAX_PASSWORD_LENGTH: usize = 128;

const UPPERCASE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const LOWERCASE: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
const NUMBERS: &[u8] = b"0123456789";
const SYMBOLS: &[u8] = b"!@#$%^&*()_+-=[]{}|;':\",.<>/?`~";
/// Left out unless `--include-ambiguous` is given.
const AMBIGUOUS: &[u8] = b"O0l1";
/// Left out unless `--include-similar` is given.
const SIMILAR: &[u8] = b"il1";

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
                "Include ambiguous characters O, 0, l, 1, which are left out by default.",
                None,
            )
            .switch(
                "include-similar",
                "Include similar characters i, l, 1, which are left out by default.",
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

        let chars: Option<Spanned<i64>> = call.get_flag(engine_state, stack, "chars")?;
        let no_uppercase = call.has_flag(engine_state, stack, "no-uppercase")?;
        let no_lowercase = call.has_flag(engine_state, stack, "no-lowercase")?;
        let no_numbers = call.has_flag(engine_state, stack, "no-numbers")?;
        let no_symbols = call.has_flag(engine_state, stack, "no-symbols")?;
        let require_each_type = call.has_flag(engine_state, stack, "require-each-type")?;
        let include_ambiguous = call.has_flag(engine_state, stack, "include-ambiguous")?;
        let include_similar = call.has_flag(engine_state, stack, "include-similar")?;

        let keep = |c: &u8| {
            let ambiguous = AMBIGUOUS.contains(c);
            let similar = SIMILAR.contains(c);
            !(ambiguous || similar)
                || (ambiguous && include_ambiguous)
                || (similar && include_similar)
        };
        let sets: Vec<Vec<u8>> = [
            (!no_uppercase, UPPERCASE),
            (!no_lowercase, LOWERCASE),
            (!no_numbers, NUMBERS),
            (!no_symbols, SYMBOLS),
        ]
        .into_iter()
        .filter(|(enabled, _)| *enabled)
        .map(|(_, set)| set.iter().copied().filter(keep).collect())
        .collect();

        let error = |msg: &'static str, span: Span| {
            ShellError::Generic(GenericError::new("Password generation failed", msg, span))
        };
        let length = match chars {
            None => DEFAULT_PASSWORD_LENGTH,
            // A negative length fails the conversion and is reported like 0.
            Some(chars) => match usize::try_from(chars.item) {
                Ok(0) | Err(_) => {
                    return Err(error("Password length must be greater than 0", chars.span));
                }
                Ok(length) if length > MAX_PASSWORD_LENGTH => {
                    return Err(error(
                        "Password length must be less than or equal to 128",
                        chars.span,
                    ));
                }
                Ok(length) => length,
            },
        };
        if require_each_type && sets.len() < 4 {
            return Err(error(
                "Invalid password configuration: You've enabled `require_each_type`, but have \
                 also excluded one or more character sets.",
                span,
            ));
        }
        if require_each_type && length < sets.len() {
            return Err(error(
                "Password length must be at least 4 to include each character type",
                span,
            ));
        }
        if sets.is_empty() {
            return Err(error(
                "Invalid password configuration: At least one character type must be included.",
                span,
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

/// Builds a password of `length` characters from `sets`, each of which must be non-empty.
///
/// When `length` leaves room for it, the password starts with one character from every set.
/// The rest comes from all sets combined, and the result is shuffled.
fn generate_password(length: usize, sets: &[Vec<u8>]) -> String {
    let mut rng = rand::rng();
    let mut password: Vec<u8> = if length >= sets.len() {
        sets.iter()
            .filter_map(|set| set.choose(&mut rng).copied())
            .collect()
    } else {
        Vec::with_capacity(length)
    };
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
