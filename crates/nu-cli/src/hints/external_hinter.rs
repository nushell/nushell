use log::error;
use nu_ansi_term::Style;
use nu_engine::ClosureEvalOnce;
use nu_protocol::{
    PipelineData, Record, Span, Value,
    engine::{Closure, EngineState, Stack},
};
use reedline::{Hinter, History};
use std::sync::Arc;
use unicode_segmentation::UnicodeSegmentation;

pub(crate) struct ExternalHinter {
    engine_state: Arc<EngineState>,
    stack: Arc<Stack>,
    closure: Closure,
    style: Style,
    current_hint: String,
}

impl ExternalHinter {
    pub(crate) fn new(
        engine_state: Arc<EngineState>,
        stack: Arc<Stack>,
        closure: Closure,
        style: Style,
    ) -> Self {
        Self {
            engine_state,
            stack,
            closure,
            style,
            current_hint: String::new(),
        }
    }

    fn evaluate_external_hint(
        &self,
        line: &str,
        pos: usize,
        cwd: &str,
    ) -> Result<Option<String>, String> {
        let span = Span::unknown();
        let context = Record::from_raw_cols_vals(
            vec!["line".to_string(), "pos".to_string(), "cwd".to_string()],
            vec![
                Value::string(line, span),
                Value::int(pos as i64, span),
                Value::string(cwd, span),
            ],
            span,
            span,
        )
        .map_err(|err| format!("failed to build context record: {err}"))?;

        let stack = Stack::with_parent(self.stack.clone());
        let result = ClosureEvalOnce::new(self.engine_state.as_ref(), &stack, self.closure.clone())
            .add_arg(Value::record(context, span))
            .run_with_input(PipelineData::empty())
            .and_then(|data| data.into_value(span));

        match result {
            Ok(Value::String { val, .. }) => Ok(Some(val)),
            Ok(Value::Nothing { .. }) => Ok(None),
            Ok(value) => {
                error!(
                    "external_hinter: unsupported closure return type {}",
                    value.get_type()
                );
                Ok(None)
            }
            Err(err) => Err(format!("closure evaluation failed: {err}")),
        }
    }
}

impl Hinter for ExternalHinter {
    fn handle(
        &mut self,
        line: &str,
        pos: usize,
        _history: &dyn History,
        use_ansi_coloring: bool,
        cwd: &str,
    ) -> String {
        self.current_hint = match self.evaluate_external_hint(line, pos, cwd) {
            Ok(Some(hint)) => hint,
            Ok(None) => String::new(),
            Err(err) => {
                error!("external_hinter: {err}");
                String::new()
            }
        };

        if use_ansi_coloring && !self.current_hint.is_empty() {
            self.style.paint(&self.current_hint).to_string()
        } else {
            self.current_hint.clone()
        }
    }

    fn complete_hint(&self) -> String {
        self.current_hint.clone()
    }

    fn next_hint_token(&self) -> String {
        first_hint_token(&self.current_hint)
    }
}

fn first_hint_token(hint: &str) -> String {
    let mut reached_content = false;
    hint.split_word_bounds()
        .take_while(
            |word| match (word.chars().all(char::is_whitespace), reached_content) {
                (_, true) => false,
                (true, false) => true,
                (false, false) => {
                    reached_content = true;
                    true
                }
            },
        )
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_parser::parse;
    use nu_protocol::{ast::Expr, engine::StateWorkingSet};
    use reedline::FileBackedHistory;

    fn history() -> FileBackedHistory {
        match FileBackedHistory::new(10) {
            Ok(history) => history,
            Err(err) => panic!("failed to build history: {err}"),
        }
    }

    fn parse_test_closure(source: &str) -> (Arc<EngineState>, Closure) {
        let mut engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);
        let parsed = parse(&mut working_set, None, source.as_bytes(), false);
        assert!(
            working_set.parse_errors.is_empty(),
            "unexpected parse errors: {:?}",
            working_set.parse_errors
        );

        let Some(pipeline) = parsed.pipelines.first() else {
            panic!("expected one pipeline in parsed source");
        };
        let Some(element) = pipeline.elements.first() else {
            panic!("expected one pipeline element in parsed source");
        };

        let block_id = match element.expr.expr {
            Expr::Closure(block_id) => block_id,
            _ => panic!("source did not parse to a closure expression"),
        };

        if let Err(err) = engine_state.merge_delta(working_set.render()) {
            panic!("failed to merge delta: {err}");
        }

        (
            Arc::new(engine_state),
            Closure {
                block_id,
                captures: vec![],
            },
        )
    }

    #[test]
    fn uses_external_hint_string_and_caches_for_completion() {
        let (engine_state, closure) = parse_test_closure("{|ctx| 'hello there'}");
        let mut hinter =
            ExternalHinter::new(engine_state, Arc::new(Stack::new()), closure, Style::new());
        let history = history();

        let hint = hinter.handle("echo ", 5, &history, false, "/tmp");
        assert_eq!(hint, "hello there");
        assert_eq!(hinter.complete_hint(), "hello there");
        assert_eq!(hinter.next_hint_token(), "hello");
    }

    #[test]
    fn null_result_returns_no_hint() {
        let (engine_state, closure) = parse_test_closure("{|ctx| null}");
        let mut hinter =
            ExternalHinter::new(engine_state, Arc::new(Stack::new()), closure, Style::new());
        let history = history();

        let hint = hinter.handle("echo", 4, &history, false, "/tmp");
        assert_eq!(hint, "");
        assert_eq!(hinter.complete_hint(), "");
        assert_eq!(hinter.next_hint_token(), "");
    }

    #[test]
    fn invalid_return_type_returns_no_hint() {
        let (engine_state, closure) = parse_test_closure("{|ctx| 42}");
        let mut hinter =
            ExternalHinter::new(engine_state, Arc::new(Stack::new()), closure, Style::new());
        let history = history();

        let hint = hinter.handle("echo", 4, &history, false, "/tmp");
        assert_eq!(hint, "");
        assert_eq!(hinter.complete_hint(), "");
    }

    #[test]
    fn eval_error_returns_no_hint() {
        let (engine_state, closure) = parse_test_closure("{|ctx| 1 / 0}");
        let mut hinter =
            ExternalHinter::new(engine_state, Arc::new(Stack::new()), closure, Style::new());
        let history = history();

        let hint = hinter.handle("echo", 4, &history, false, "/tmp");
        assert_eq!(hint, "");
        assert_eq!(hinter.complete_hint(), "");
    }

    #[test]
    fn first_hint_token_empty() {
        assert_eq!(super::first_hint_token(""), "");
    }

    #[test]
    fn first_hint_token_with_leading_whitespace() {
        assert_eq!(super::first_hint_token("   hello world"), "   hello");
    }

    #[test]
    fn first_hint_token_single_word() {
        assert_eq!(super::first_hint_token("hello"), "hello");
    }

    #[test]
    fn first_hint_token_only_whitespace() {
        assert_eq!(super::first_hint_token("   "), "   ");
    }
}
