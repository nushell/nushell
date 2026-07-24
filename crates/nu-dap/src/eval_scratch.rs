//! Expression evaluation while paused — without touching the paused engine.
//!
//! The Debugger trait never exposes the `Stack`, and the real `EngineState`
//! is mutably unavailable while the eval thread sits inside a callback. So
//! watch/hover/console expressions (and breakpoint conditions / logpoint
//! interpolations) run against a *separate* scratch engine: the expression
//! is parsed with the current shadow variables pre-declared, their captured
//! values are bound on a fresh stack, and the block runs undebugged.
//!
//! Honest limitations: custom commands from the debugged script are not
//! visible here, mutations don't affect the real program, and variables
//! whose values couldn't be shadow-captured (streams) evaluate to their
//! placeholder.

use nu_protocol::debugger::WithoutDebug;
use nu_protocol::engine::{Stack, StateWorkingSet};
use nu_protocol::{PipelineData, Span, Type, Value};

pub(crate) struct Scratch {
    engine: nu_protocol::engine::EngineState,
}

impl std::fmt::Debug for Scratch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Scratch")
    }
}

impl Scratch {
    pub(crate) fn new() -> Self {
        let mut engine = nu_cmd_lang::create_default_context();
        engine = nu_command::add_shell_command_context(engine);
        engine.set_signals(nu_protocol::Signals::new(std::sync::Arc::new(
            std::sync::atomic::AtomicBool::new(false),
        )));
        // Seed the environment so console expressions can run externals and
        // path-dependent commands — e.g. re-running a pipeline stage by hand
        // to inspect data that only exists as a stream in the debuggee.
        for (k, v) in std::env::vars() {
            if k.eq_ignore_ascii_case("pwd") {
                continue;
            }
            let value = if k.eq_ignore_ascii_case("path") {
                Value::list(
                    std::env::split_paths(&v)
                        .map(|p| Value::string(p.to_string_lossy(), Span::unknown()))
                        .collect(),
                    Span::unknown(),
                )
            } else {
                Value::string(v, Span::unknown())
            };
            engine.add_env_var(k, value);
        }
        if let Ok(cwd) = std::env::current_dir() {
            engine.add_env_var(
                "PWD".into(),
                Value::string(cwd.to_string_lossy(), Span::unknown()),
            );
        }
        Self { engine }
    }

    /// Evaluate `expr` with the given variables in scope.
    pub(crate) fn eval(&mut self, expr: &str, vars: &[(String, Value)]) -> Result<Value, String> {
        let block = {
            let mut working_set = StateWorkingSet::new(&self.engine);
            // Declare each shadow variable so the parser resolves `$name`.
            // nu registers variable names WITH the `$` prefix.
            let ids: Vec<_> = vars
                .iter()
                .map(|(name, _)| {
                    working_set.add_variable(
                        format!("${name}").into_bytes(),
                        Span::unknown(),
                        Type::Any,
                        false,
                    )
                })
                .collect();
            let block = nu_parser::parse(&mut working_set, None, expr.as_bytes(), false);
            if let Some(err) = working_set.parse_errors.first() {
                return Err(format!("{err:?}"));
            }
            let delta = working_set.render();
            self.engine
                .merge_delta(delta)
                .map_err(|e| format!("{e:?}"))?;
            (block, ids)
        };
        let (block, ids) = block;

        let mut stack = Stack::new();
        for (id, (_, value)) in ids.iter().zip(vars) {
            stack.add_var(*id, value.clone());
        }
        match nu_engine::eval_block::<WithoutDebug>(
            &self.engine,
            &mut stack,
            &block,
            PipelineData::empty(),
        ) {
            Ok(exec) => exec
                .body
                .into_value(Span::unknown())
                .map_err(|e| format!("{e}")),
            Err(e) => Err(format!("{e}")),
        }
    }
}

/// Interpolate a DAP logpoint message: text with `{expression}` segments.
/// Unmatched braces are passed through verbatim.
pub(crate) fn interpolate(
    scratch: &mut Scratch,
    template: &str,
    vars: &[(String, Value)],
) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('}') {
            Some(end) => {
                let expr = &after[..end];
                match scratch.eval(expr, vars) {
                    Ok(Value::String { val, .. }) => out.push_str(&val),
                    Ok(v) => {
                        out.push_str(&v.to_expanded_string(", ", &nu_protocol::Config::default()))
                    }
                    Err(e) => {
                        let _ = write!(out, "{{error: {e}}}");
                    }
                }
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
