use nu_parser::parse;
use nu_protocol::{
    ParseError,
    engine::{EngineState, StateWorkingSet},
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

        // Unclosed delimiters and unexpected EOF both mean the user may still be
        // typing a multi-line construct (e.g. an open `{` in the REPL).
        //
        // Lex presentation may reshape some stack failures (extra `}` / `)` / `]`)
        // into `LabeledErrorWithHelp` ("Missing `{`/`[`/`(`…"). Those are complete
        // but wrong inputs, not mid-edit unclosed forms — keep them Complete so the
        // REPL submits the line and shows the diagnostic instead of waiting for more
        // input. Only extend Incomplete if a new reshape intentionally represents
        // a still-open multi-line edit.
        if matches!(
            working_set.parse_errors.first(),
            Some(ParseError::UnexpectedEof(..) | ParseError::Unclosed(..))
        ) {
            ValidationResult::Incomplete
        } else {
            ValidationResult::Complete
        }
    }
}
