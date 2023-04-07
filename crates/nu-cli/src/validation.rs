use nu_parser::parse;
use nu_protocol::{
    engine::{EngineState, StateWorkingSet},
    ParseError,
};
use reedline::{ValidationResult, Validator};
use std::sync::Arc;

pub struct NuValidator {
    pub engine_state: Arc<EngineState>,
}

impl Validator for NuValidator {
    fn validate(&self, line: &str) -> ValidationResult {
        let mut working_set = StateWorkingSet::new(&self.engine_state);
        parse(&mut working_set, None, line.as_bytes(), false);

        if matches!(
            working_set.parse_errors.first(),
            Some(ParseError::UnexpectedEof(..))
        ) {
            ValidationResult::Incomplete
        } else {
            ValidationResult::Complete
        }
    }
}
