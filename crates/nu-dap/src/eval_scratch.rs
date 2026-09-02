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
pub(crate) fn env_value(key: &str, raw: String) -> Value {
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
    pub(crate) fn engine_footprint(&self) -> (usize, usize) {
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
pub(crate) fn is_nu_interpolation(trimmed: &str) -> bool {
    trimmed.len() > 2
        && ((trimmed.starts_with("$\"") && trimmed.ends_with('"'))
            || (trimmed.starts_with("$'") && trimmed.ends_with('\'')))
}
