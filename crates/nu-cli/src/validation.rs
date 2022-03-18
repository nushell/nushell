use nu_parser::{parse, ParseError};
use nu_protocol::engine::{EngineState, StateWorkingSet};
use reedline::{ValidationResult, Validator};

pub struct NuValidator {
    pub engine_state: EngineState,
}

impl Validator for NuValidator {
    fn validate(&self, line: &str) -> ValidationResult {
        let mut working_set = StateWorkingSet::new(&self.engine_state);
        let (_, err) = parse(&mut working_set, None, line.as_bytes(), false, &[]);

        if matches!(err, Some(ParseError::UnexpectedEof(..))) {
            ValidationResult::Incomplete
        } else {
            ValidationResult::Complete
        }
    }
}
