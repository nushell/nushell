//! Expression evaluation while paused — without re-entering the paused engine.
//!
//! The real `EngineState` can't be used to evaluate while the eval thread sits
//! inside a Debugger callback, so watch/hover/console expressions (and
//! breakpoint conditions / logpoint interpolation) run against a *separate*
//! scratch engine: the expression is parsed with the snapshotted shadow
//! variables pre-declared, their values bound on a fresh stack, run undebugged.
//!
//! Limitations: the script's own commands aren't visible here, mutations don't
//! affect the real program, and stream-valued variables show their placeholder.

use nu_protocol::ast::Block;
use nu_protocol::debugger::WithoutDebug;
use nu_protocol::engine::{EngineState, Stack, StateWorkingSet};
use nu_protocol::{PipelineData, Span, Type, Value, VarId};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(derive_more::Debug)]
pub(crate) struct Scratch {
    #[debug(skip)]
    engine: EngineState,
    /// Shadow variables already declared in `engine`. Names are only ever
    /// added, so a `VarId` once handed out stays valid for the rest of the
    /// session — which is what lets `blocks` below be reused.
    var_ids: HashMap<String, VarId>,
    /// Successfully parsed expressions, keyed by their source text. Declaring a
    /// variable or parsing an expression merges a delta into `engine`, and
    /// nothing is ever reclaimed, so without these two caches a logpoint on a
    /// hot line would grow the engine on every hit.
    ///
    /// Failures are deliberately not cached: an expression that fails to parse
    /// now may reference a variable that gets declared later.
    blocks: HashMap<String, Arc<Block>>,
}

// --- Construction ---------------------------------------------------------

impl Scratch {
    pub(crate) fn new() -> Self {
        let mut engine = nu_cmd_lang::create_default_context();

        engine = nu_command::add_shell_command_context(engine);

        engine.set_signals(nu_protocol::Signals::new(Arc::new(
            std::sync::atomic::AtomicBool::new(false),
        )));

        Self::seed_env(&mut engine);

        Self {
            engine,
            var_ids: HashMap::new(),
            blocks: HashMap::new(),
        }
    }

    /// Seed the environment from our own process so console expressions can run
    /// externals and path-dependent commands (e.g. re-run a pipeline stage by
    /// hand). `PWD` is skipped and then set from the real cwd, because nu
    /// requires it to be absolute and canonical.
    fn seed_env(engine: &mut EngineState) {
        for (key, raw) in std::env::vars() {
            if key.eq_ignore_ascii_case("pwd") {
                continue;
            }

            let value = env_value(&key, raw);
            engine.add_env_var(key, value);
        }

        if let Ok(cwd) = std::env::current_dir() {
            engine.add_env_var(
                "PWD".into(),
                Value::string(cwd.to_string_lossy(), Span::unknown()),
            );
        }
    }
}

/// One environment variable's nu value. `PATH` becomes a list, the way nu's own
/// startup presents it, so `$env.PATH | each { … }` works here too.
fn env_value(key: &str, raw: String) -> Value {
    if key.eq_ignore_ascii_case("path") {
        Value::list(
            std::env::split_paths(&raw)
                .map(|p| Value::string(p.to_string_lossy(), Span::unknown()))
                .collect(),
            Span::unknown(),
        )
    } else {
        Value::string(raw, Span::unknown())
    }
}

// --- Evaluation -----------------------------------------------------------

impl Scratch {
    /// Evaluate `expr` with the given variables in scope.
    pub(crate) fn eval(&mut self, expr: &str, vars: &[(String, Value)]) -> Result<Value, String> {
        self.declare_missing(vars)?;
        let block = self.block_for(expr)?;
        let mut stack = self.bind(vars);

        let exec = nu_engine::eval_block::<WithoutDebug>(
            &self.engine,
            &mut stack,
            &block,
            PipelineData::empty(),
        )
        .map_err(|e| e.to_string())?;

        exec.body
            .into_value(Span::unknown())
            .map_err(|e| e.to_string())
    }

    /// Declare any shadow variable the parser hasn't seen yet, so it can resolve
    /// `$name`. Costs nothing once a frame's locals have been declared once,
    /// which is the steady state for watch expressions and logpoints.
    fn declare_missing(&mut self, vars: &[(String, Value)]) -> Result<(), String> {
        let missing: Vec<&str> = vars
            .iter()
            .map(|(name, _)| name.as_str())
            .filter(|name| !self.var_ids.contains_key(*name))
            .collect();

        if missing.is_empty() {
            return Ok(());
        }

        let mut working_set = StateWorkingSet::new(&self.engine);
        // nu registers variable names WITH the `$` prefix (`add_variable`
        // prepends one anyway, but say so at the call site).
        let declared: Vec<(String, VarId)> = missing
            .into_iter()
            .map(|name| {
                let id = working_set.add_variable(
                    format!("${name}").into_bytes(),
                    Span::unknown(),
                    Type::Any,
                    false,
                );
                (name.to_string(), id)
            })
            .collect();

        let delta = working_set.render();
        self.engine.merge_delta(delta).map_err(|e| e.to_string())?;
        self.var_ids.extend(declared);
        Ok(())
    }

    /// The parsed block for `expr`, parsing and caching it on first use.
    fn block_for(&mut self, expr: &str) -> Result<Arc<Block>, String> {
        if let Some(block) = self.blocks.get(expr) {
            return Ok(Arc::clone(block));
        }

        let mut working_set = StateWorkingSet::new(&self.engine);
        let block = nu_parser::parse(&mut working_set, None, expr.as_bytes(), false);

        // Bail before merging, so a broken condition re-evaluated on every hit
        // leaves nothing behind.
        if let Some(err) = working_set.parse_errors.first() {
            return Err(err.to_string());
        }

        if let Some(err) = working_set.compile_errors.first() {
            return Err(err.to_string());
        }

        let delta = working_set.render();
        self.engine.merge_delta(delta).map_err(|e| e.to_string())?;
        self.blocks.insert(expr.to_string(), Arc::clone(&block));

        Ok(block)
    }

    /// How much permanent state the engine holds: (variables, blocks). Lets the
    /// tests assert that re-evaluating an expression allocates nothing new,
    /// which is the whole point of `var_ids` and `blocks`.
    #[cfg(test)]
    fn engine_footprint(&self) -> (usize, usize) {
        (self.engine.num_vars(), self.engine.num_blocks())
    }

    /// A fresh stack with each shadow value bound to its variable.
    fn bind(&self, vars: &[(String, Value)]) -> Stack {
        let mut stack = Stack::new();
        for (name, value) in vars {
            if let Some(id) = self.var_ids.get(name) {
                stack.add_var(*id, value.clone());
            }
        }
        stack
    }
}

// --- Interpolation --------------------------------------------------------

impl Scratch {
    /// Interpolate a logpoint message. Two syntaxes are accepted:
    /// - **Nushell** — if the whole message is a `$"..."` / `$'...'`
    ///   interpolation literal, it's evaluated as-is, so nu users write
    ///   `$"iteration ($i)"` the way they would in a script.
    /// - **DAP `{expression}`** — otherwise each `{expr}` segment is evaluated
    ///   and substituted; unmatched braces pass through verbatim.
    pub(crate) fn interpolate(&mut self, template: &str, vars: &[(String, Value)]) -> String {
        let trimmed = template.trim();
        if is_nu_interpolation(trimmed) {
            return self.eval_to_string(trimmed, vars);
        }

        self.interpolate_braces(template, vars)
    }

    /// Substitute every `{expr}` segment; an unmatched `{` is literal text.
    fn interpolate_braces(&mut self, template: &str, vars: &[(String, Value)]) -> String {
        let mut out = String::new();
        let mut rest = template;

        while let Some(start) = rest.find('{') {
            out.push_str(&rest[..start]);
            let after = &rest[start + 1..];
            match after.find('}') {
                Some(end) => {
                    out.push_str(&self.eval_to_string(&after[..end], vars));
                    rest = &after[end + 1..];
                }
                None => {
                    out.push('{');
                    rest = after;
                }
            }
        }

        out.push_str(rest);
        out
    }

    /// Evaluate for display: strings pass through unquoted, anything else is
    /// rendered, and a failure becomes a visible placeholder rather than
    /// swallowing the message — a logpoint never pauses, so an invisible error
    /// would leave nothing to show that anything went wrong.
    fn eval_to_string(&mut self, expr: &str, vars: &[(String, Value)]) -> String {
        match self.eval(expr, vars) {
            Ok(Value::String { val, .. }) => val,
            Ok(v) => v.to_expanded_string(", ", &nu_protocol::Config::default()),
            Err(e) => format!("{{error: {e}}}"),
        }
    }
}

/// Whether the whole message is a nu string-interpolation literal, in which case
/// it is evaluated as one expression so `($expr)` segments interpolate the
/// native nu way.
///
/// A heuristic: it checks the delimiters, not the structure, so something like
/// `$"a" $"b"` is misread as a single literal. The fallback is a visible
/// `{error: …}` from the evaluation, and doing better means real nu lexing.
fn is_nu_interpolation(trimmed: &str) -> bool {
    trimmed.len() > 2
        && ((trimmed.starts_with("$\"") && trimmed.ends_with('"'))
            || (trimmed.starts_with("$'") && trimmed.ends_with('\'')))
}

#[cfg(test)]
mod tests {
    //! Unit tests for [`crate::eval_scratch`].

    use super::{Scratch, env_value, is_nu_interpolation};
    use nu_protocol::{Span, Value};
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    fn sp() -> Span {
        Span::unknown()
    }

    /// One shadow variable, the shape `eval`/`interpolate` take.
    fn vars(name: &str, value: Value) -> Vec<(String, Value)> {
        vec![(name.to_string(), value)]
    }

    #[test]
    fn path_becomes_a_list_other_vars_stay_strings() {
        let joined = std::env::join_paths(["/one", "/two"].iter().map(std::path::Path::new))
            .expect("joinable");
        let value = env_value("PATH", joined.to_string_lossy().to_string());
        let list = value.as_list().expect("PATH is a list");
        assert_eq!(list.len(), 2);

        // Case-insensitive: Windows spells it `Path`.
        assert!(env_value("Path", "/one".to_string()).as_list().is_ok());

        let other = env_value("EDITOR", "hx".to_string());
        assert_eq!(other, Value::string("hx", sp()));
    }

    #[test]
    fn eval_binds_shadow_variables() {
        let mut scratch = Scratch::new();
        let result = scratch
            .eval("$x + 1", &vars("x", Value::int(1, sp())))
            .expect("evaluates");
        assert_eq!(result, Value::int(2, sp()));
    }

    /// A parse error reaches the watch UI as a sentence. Rendering it with `Debug`
    /// leaks the internal shape (`Span { start: .., end: .. }`) into the message.
    #[test]
    fn parse_errors_are_displayed_not_debugged() {
        let mut scratch = Scratch::new();
        let err = scratch
            .eval("$x +", &vars("x", Value::int(1, sp())))
            .expect_err("incomplete");
        assert!(
            !err.contains("Span {"),
            "message should not be a Debug dump: {err}"
        );
    }

    /// A watch expression can parse and still fail to compile. The message has to
    /// be nushell's own, not the "missing compiled representation" that
    /// `eval_block` reports for a block whose IR never got built.
    #[test]
    fn compile_errors_reach_the_watch_pane() {
        let mut scratch = Scratch::new();
        let err = scratch
            .eval("$env.PWD = \"/tmp\"", &[])
            .expect_err("cannot set PWD");

        assert!(
            err.contains("PWD cannot be set manually"),
            "unexpected message: {err}"
        );
    }

    /// The point of the `var_ids` / `blocks` caches: a logpoint on a hot line calls
    /// `eval` once per hit, and every declaration or parse merges a delta into the
    /// engine permanently. Re-evaluating the same expression must add nothing.
    #[test]
    fn repeat_evaluation_does_not_grow_the_engine() {
        let mut scratch = Scratch::new();
        let v = vars("x", Value::int(1, sp()));

        scratch.eval("$x + 1", &v).expect("evaluates");
        let before = scratch.engine_footprint();

        for _ in 0..5 {
            scratch.eval("$x + 1", &v).expect("evaluates");
        }

        assert_eq!(before, scratch.engine_footprint());
    }

    /// A failing expression must not leave a merged delta behind either — a broken
    /// breakpoint condition is re-evaluated on every hit.
    #[test]
    fn repeat_failure_does_not_grow_the_engine() {
        let mut scratch = Scratch::new();
        let v = vars("x", Value::int(1, sp()));

        scratch.eval("$x +", &v).expect_err("incomplete");
        let before = scratch.engine_footprint();

        for _ in 0..5 {
            scratch.eval("$x +", &v).expect_err("incomplete");
        }

        assert_eq!(before, scratch.engine_footprint());
    }

    #[rstest]
    #[case::double_quoted("$\"a ($x)\"", true)]
    #[case::single_quoted("$'a'", true)]
    #[case::bare_text("plain text", false)]
    #[case::dap_braces_are_not_nu("iteration {x}", false)]
    // Only the delimiters are checked, so `$"` alone is too short to qualify.
    #[case::unterminated_opener("$\"", false)]
    fn nu_interpolation_is_recognised_by_its_delimiters(
        #[case] input: &str,
        #[case] expected: bool,
    ) {
        assert_eq!(is_nu_interpolation(input), expected);
    }

    #[test]
    fn interpolate_evaluates_a_whole_nu_literal() {
        let mut scratch = Scratch::new();
        let out = scratch.interpolate("$\"i is ($x)\"", &vars("x", Value::int(7, sp())));
        assert_eq!(out, "i is 7");
    }

    #[test]
    fn interpolate_substitutes_dap_brace_segments() {
        let mut scratch = Scratch::new();
        let out = scratch.interpolate("i is {$x} now", &vars("x", Value::int(7, sp())));
        assert_eq!(out, "i is 7 now");
    }

    /// An unmatched brace is literal text, so a message that merely mentions `{`
    /// still logs.
    #[test]
    fn interpolate_passes_unmatched_braces_through() {
        let mut scratch = Scratch::new();
        let out = scratch.interpolate("a { b", &[]);
        assert_eq!(out, "a { b");
    }

    /// A logpoint never pauses, so a failed segment has to be visible in the
    /// message — swallowing it would leave no sign anything went wrong.
    #[test]
    fn interpolate_shows_a_placeholder_for_a_failed_segment() {
        let mut scratch = Scratch::new();
        let out = scratch.interpolate("value {$nope}", &[]);
        assert!(
            out.starts_with("value {error: "),
            "expected an error placeholder, got: {out}"
        );
    }
}
