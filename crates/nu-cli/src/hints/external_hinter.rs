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

/// The result of evaluating the external hinter closure.
///
/// Closures may return a plain string (just the hint) or a record
/// `{hint: string, next_token?: string}` when the default whitespace-based
/// token splitting is not appropriate for the hint text.
struct HintResult {
    /// The hint suffix to display after the cursor.
    hint: String,
    /// Optional override for the next token returned by `next_hint_token`.
    /// When `None`, the default `first_hint_token` splitter is used.
    next_token: Option<String>,
}

pub(crate) struct ExternalHinter {
    engine_state: Arc<EngineState>,
    stack: Arc<Stack>,
    closure: Closure,
    style: Style,
    current_hint: String,
    current_next_token: Option<String>,
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
            current_next_token: None,
        }
    }

    fn evaluate_external_hint(
        &self,
        line: &str,
        pos: usize,
        cwd: &str,
    ) -> Result<Option<HintResult>, String> {
        // No call span available — this is a reedline hinter callback
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

        // Reuse completion-worker isolation so external commands cannot inherit
        // Reedline's terminal through stdin, stdout or stderr.
        let stack = Stack::with_parent(self.stack.clone())
            .suppress_output()
            .suppress_stdin();
        let result = ClosureEvalOnce::try_new(
            self.engine_state.as_ref(),
            &stack,
            self.closure.clone(),
            span,
        )
        .and_then(|eval| eval.add_arg(Value::record(context, span)))
        .and_then(|closure| closure.run_with_input(PipelineData::empty()))
        .and_then(|data| data.into_value(span));

        match result {
            Ok(Value::String { val, .. }) => Ok(Some(HintResult {
                hint: val,
                next_token: None,
            })),
            Ok(Value::Record { val, .. }) => {
                let hint = val
                    .get("hint")
                    .and_then(|v| v.as_str().ok())
                    .map(|s| s.to_string());
                let next_token = val
                    .get("next_token")
                    .and_then(|v| v.as_str().ok())
                    .map(|s| s.to_string());
                match hint {
                    Some(hint) => Ok(Some(HintResult { hint, next_token })),
                    None => {
                        error!("external_hinter: record return must contain a 'hint' string field");
                        Ok(None)
                    }
                }
            }
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
        match self.evaluate_external_hint(line, pos, cwd) {
            Ok(Some(result)) => {
                self.current_next_token = result.next_token;
                self.current_hint = result.hint;
            }
            Ok(None) => {
                self.current_next_token = None;
                self.current_hint = String::new();
            }
            Err(err) => {
                error!("external_hinter: {err}");
                self.current_next_token = None;
                self.current_hint = String::new();
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
        if let Some(ref next_token) = self.current_next_token {
            next_token.clone()
        } else {
            first_hint_token(&self.current_hint)
        }
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
        FileBackedHistory::new(10).unwrap()
    }

    fn create_hinter(source: &str) -> ExternalHinter {
        create_hinter_with_engine(source, EngineState::new())
    }

    fn create_hinter_with_engine(source: &str, mut engine_state: EngineState) -> ExternalHinter {
        let mut working_set = StateWorkingSet::new(&engine_state);
        let parsed = parse(&mut working_set, None, source.as_bytes(), false);
        assert!(
            working_set.parse_errors.is_empty(),
            "unexpected parse errors: {:?}",
            working_set.parse_errors
        );

        let pipeline = parsed.pipelines.first().unwrap();
        let element = pipeline.elements.first().unwrap();

        let Expr::Closure(block_id) = element.expr.expr else {
            panic!("source did not parse to a closure expression");
        };

        engine_state.merge_delta(working_set.render()).unwrap();

        let closure = Closure {
            block_id,
            captures: vec![],
        };

        ExternalHinter::new(
            Arc::new(engine_state),
            Arc::new(Stack::new()),
            closure,
            Style::new(),
        )
    }

    #[cfg(unix)]
    fn create_external_command_hinter(source: &str) -> ExternalHinter {
        let mut engine_state =
            nu_command::add_shell_command_context(nu_cmd_lang::create_default_context());
        crate::gather_parent_env_vars(&mut engine_state, &std::env::current_dir().unwrap());
        create_hinter_with_engine(source, engine_state)
    }

    #[cfg(unix)]
    #[test]
    fn external_command_is_detached_from_the_shell() {
        // A detached child starts a new session and process group even when the
        // parent is not interactive, as is typical when running tests in CI.
        let mut hinter = create_external_command_hinter(
            r#"{|ctx| ^sh -c 'pgid=$(ps -o pgid= -p $$); if [ "$((pgid))" = "$$" ]; then printf isolated; else printf attached; fi'}"#,
        );

        let hint = hinter.handle("", 0, &history(), false, "/tmp");

        assert_eq!(hint, "isolated");
        assert!(!hinter.stack.suppress_stdin);
    }

    #[cfg(unix)]
    #[test]
    fn detached_external_command_still_receives_pipeline_input() {
        let mut hinter = create_external_command_hinter(
            r#"{|ctx| 'hint text' | ^sh -c 'pgid=$(ps -o pgid= -p $$); if [ "$((pgid))" = "$$" ]; then cat; else printf attached; fi'}"#,
        );

        let hint = hinter.handle("", 0, &history(), false, "/tmp");

        assert_eq!(hint.trim_end(), "hint text");
    }

    #[cfg(unix)]
    #[test]
    fn external_command_stderr_is_off_the_terminal() {
        let mut hinter = create_external_command_hinter(
            "{|ctx| ^sh -c 'if [ /dev/fd/2 -ef /dev/null ]; then printf isolated; else printf attached; fi'}",
        );

        assert_eq!(hinter.handle("", 0, &history(), false, "/tmp"), "isolated");
    }

    #[cfg(unix)]
    #[test]
    fn intermediate_external_command_stdout_is_off_the_terminal() {
        let mut hinter = create_external_command_hinter(
            "{|ctx| ^sh -c 'test /dev/fd/1 -ef /dev/null'; $env.LAST_EXIT_CODE | into string}",
        );

        assert_eq!(hinter.handle("", 0, &history(), false, "/tmp"), "0");
    }

    #[cfg(unix)]
    #[test]
    fn external_command_stderr_can_still_be_captured() {
        for source in [
            "{|ctx| ^sh -c 'printf hint-error >&2' | complete | get stderr}",
            "{|ctx| ^sh -c 'printf hint-error >&2' e>| str trim}",
        ] {
            let mut hinter = create_external_command_hinter(source);

            assert_eq!(
                hinter.handle("", 0, &history(), false, "/tmp"),
                "hint-error",
                "{source}"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn input_declines_instead_of_reading_the_terminal() {
        let mut hinter =
            create_external_command_hinter("{|ctx| try { input; 'read' } catch {|e| $e.msg }}");

        assert_eq!(
            hinter.handle("", 0, &history(), false, "/tmp"),
            "Interactive input is unavailable in this context"
        );
    }

    #[test]
    fn uses_external_hint_string_and_caches_for_completion() {
        let mut hinter = create_hinter("{|ctx| 'hello there'}");
        let history = history();

        let hint = hinter.handle("echo ", 5, &history, false, "/tmp");
        assert_eq!(hint, "hello there");
        assert_eq!(hinter.complete_hint(), "hello there");
        assert_eq!(hinter.next_hint_token(), "hello");
    }

    #[test]
    fn closure_receives_context_record() {
        let mut hinter = create_hinter("{|ctx| $ctx.line}");
        let history = history();

        let hint = hinter.handle("echo hello", 10, &history, false, "/tmp");
        assert_eq!(hint, "echo hello");
    }

    #[test]
    fn closure_receives_the_full_buffer_and_absolute_byte_position() {
        let mut hinter = create_hinter(
            r#"{|ctx| {hint: $"($ctx.line)@($ctx.pos)", next_token: $"pos=($ctx.pos)"}}"#,
        );
        let history = history();

        let hint = hinter.handle("echo 🎉)", 9, &history, false, "/tmp");

        assert_eq!(hint, "echo 🎉)@9");
        assert_eq!(hinter.next_hint_token(), "pos=9");
    }

    #[test]
    fn record_return_with_hint_and_next_token() {
        let mut hinter = create_hinter("{|ctx| {hint: 'hello there', next_token: 'hello'}}");
        let history = history();

        let hint = hinter.handle("echo ", 5, &history, false, "/tmp");
        assert_eq!(hint, "hello there");
        assert_eq!(hinter.complete_hint(), "hello there");
        assert_eq!(hinter.next_hint_token(), "hello");
    }

    #[test]
    fn record_return_with_custom_next_token() {
        let mut hinter =
            create_hinter("{|ctx| {hint: 'hello there friend', next_token: 'hello there'}}");
        let history = history();

        let hint = hinter.handle("echo ", 5, &history, false, "/tmp");
        assert_eq!(hint, "hello there friend");
        assert_eq!(hinter.next_hint_token(), "hello there");
    }

    #[test]
    fn record_return_hint_only_uses_default_next_token() {
        let mut hinter = create_hinter("{|ctx| {hint: 'hello there'}}");
        let history = history();

        let hint = hinter.handle("echo ", 5, &history, false, "/tmp");
        assert_eq!(hint, "hello there");
        assert_eq!(hinter.next_hint_token(), "hello");
    }

    #[test]
    fn null_result_returns_no_hint() {
        let mut hinter = create_hinter("{|ctx| null}");
        let history = history();

        let hint = hinter.handle("echo", 4, &history, false, "/tmp");
        assert_eq!(hint, "");
        assert_eq!(hinter.complete_hint(), "");
        assert_eq!(hinter.next_hint_token(), "");
    }

    #[test]
    fn invalid_return_type_returns_no_hint() {
        let mut hinter = create_hinter("{|ctx| 42}");
        let history = history();

        let hint = hinter.handle("echo", 4, &history, false, "/tmp");
        assert_eq!(hint, "");
        assert_eq!(hinter.complete_hint(), "");
    }

    #[test]
    fn eval_error_returns_no_hint() {
        let mut hinter = create_hinter("{|ctx| 1 / 0}");
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
