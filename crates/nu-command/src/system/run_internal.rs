use nu_engine::{CallExt, command_prelude::*, find_builtin_decl};
use nu_protocol::ir;

/// Internal command used by the `%($cmd)`/`%$cmd` dynamic builtin dispatch syntax.
///
/// The `%` sigil statically resolves a builtin at parse time. When the head is a runtime
/// expression (`%($cmd)` or `%$cmd`), the parser defers resolution and the IR compiler
/// rewrites the call as `run-internal <head-expr> ...args`. This command then looks up the
/// target builtin at runtime and enforces that it is a `CommandType::Builtin`.
#[derive(Clone)]
pub struct RunInternal;

impl Command for RunInternal {
    fn name(&self) -> &str {
        "run-internal"
    }

    fn description(&self) -> &str {
        "Run a built-in command by name. Used internally by `%($cmd)` dynamic dispatch."
    }

    fn signature(&self) -> Signature {
        Signature::build("run-internal")
            .input_output_types(vec![(Type::Any, Type::Any)])
            .required(
                "name",
                SyntaxShape::String,
                "The name of the built-in command to run.",
            )
            .rest(
                "args",
                SyntaxShape::Any,
                "Arguments to pass to the built-in command.",
            )
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let name: String = call.req(engine_state, stack, 0)?;

        let decl_id = find_builtin_decl(engine_state, &name)
            .ok_or(ShellError::CommandNotFound { span: head })?;

        let decl = engine_state.get_decl(decl_id);
        let signature = decl.signature();
        // Preserve spread markers here so `%($cmd) ...$args` forwards the same argument shape the
        // target builtin would receive in a direct call.
        let rest_args: Vec<(Value, bool)> = call.rest_preserving_spreads(engine_state, stack, 1)?;

        // Build an IR call frame for the target builtin, preserving spread arguments.
        let mut builder = ir::Call::build(decl_id, head);

        // Because `args` is parsed with `SyntaxShape::Any` at parse time (the target command
        // isn't known until runtime), flag-like tokens such as `-r` or `--recursive` arrive
        // here as plain string values instead of being resolved against the target's
        // signature the way a normal call would resolve them. Re-classify them now, mirroring
        // the parser's own long/short flag handling, so flags meant for the target builtin
        // (e.g. `run-internal cp -- -r $src $dest`) are actually recognized as flags instead of
        // being forwarded as stray positional arguments.
        let mut end_of_flags = false;
        let mut iter = rest_args.into_iter().peekable();
        while let Some((val, is_spread)) = iter.next() {
            if is_spread {
                // Skip empty spreads so builtins with no-argument defaults (e.g. `ls`
                // listing the cwd when called with no path) are not confused by an
                // empty `...$args` forwarded as an empty list.
                match val {
                    Value::List { ref vals, .. } if vals.is_empty() => continue,
                    Value::Nothing { .. } => continue,
                    _ => {
                        builder.add_spread(stack, head, val);
                    }
                }
                continue;
            }

            if !end_of_flags && let Value::String { val: s, .. } = &val {
                let span = val.span();

                // A bare `--` ends flag parsing for the target command, just like it
                // would in a normal call; everything after it is passed through as-is.
                if s == "--" {
                    end_of_flags = true;
                    continue;
                }

                if let Some(long_name) = s.strip_prefix("--") {
                    let (long_name, inline_value) = match long_name.split_once('=') {
                        Some((n, v)) => (n, Some(v)),
                        None => (long_name, None),
                    };

                    if let Some(flag) = signature.get_long_flag(long_name) {
                        add_flag_to_builder(
                            &mut builder,
                            stack,
                            &flag,
                            span,
                            inline_value.map(|v| Value::string(v, span)),
                            &mut iter,
                        )?;
                        continue;
                    }
                } else if s.starts_with('-') && s.len() > 1 {
                    let short_flags: Vec<char> = s[1..].chars().collect();
                    let resolved: Option<Vec<Flag>> = short_flags
                        .iter()
                        .map(|c| signature.get_short_flag(*c))
                        .collect();

                    if let Some(flags) = resolved {
                        // All characters in the batch matched a known short flag for the
                        // target command, so treat the whole token as a short-flag batch
                        // (e.g. `-rf`), matching how the parser treats a normal call.
                        let last = flags.len().saturating_sub(1);
                        for (i, flag) in flags.into_iter().enumerate() {
                            if i == last {
                                add_flag_to_builder(
                                    &mut builder,
                                    stack,
                                    &flag,
                                    span,
                                    None,
                                    &mut iter,
                                )?;
                            } else {
                                // Only the last flag in a batch may take a value.
                                builder.add_flag(
                                    stack,
                                    &flag.long,
                                    flag.short.map(String::from).unwrap_or_default(),
                                    span,
                                );
                            }
                        }
                        continue;
                    }
                }
            }

            builder.add_positional(stack, head, val);
        }

        // `builder.with` is a scoped guard: it registers temporary IR argument slots,
        // calls the closure, then always deallocates those slots on exit.
        builder.with(stack, |stack, engine_call| {
            decl.run(engine_state, stack, engine_call, input)
        })
    }
}

/// Add a single resolved flag to the call being built. If the flag takes a value and none was
/// supplied inline (e.g. via `--name=value`), the next non-spread rest argument is consumed as
/// the value, mirroring how the parser pulls a long/short flag's argument from the following
/// token.
fn add_flag_to_builder(
    builder: &mut ir::CallBuilder,
    stack: &mut Stack,
    flag: &Flag,
    span: Span,
    inline_value: Option<Value>,
    iter: &mut std::iter::Peekable<std::vec::IntoIter<(Value, bool)>>,
) -> Result<(), ShellError> {
    let short = flag.short.map(String::from).unwrap_or_default();

    if flag.arg.is_some() {
        let value = if let Some(value) = inline_value {
            value
        } else {
            match iter.peek() {
                Some((_, false)) => {
                    let (value, _) = iter.next().expect("peeked Some");
                    value
                }
                _ => {
                    return Err(ShellError::MissingParameter {
                        param_name: flag.long.clone(),
                        span,
                    });
                }
            }
        };
        builder.add_named(stack, &flag.long, short, span, value);
    } else {
        builder.add_flag(stack, &flag.long, short, span);
    }

    Ok(())
}
