use nu_engine::{CallExt, command_prelude::*, find_builtin_decl, get_full_help};
use nu_protocol::{
    CompareTypes,
    engine::{Argument, named_flags::expand_flag_record},
    ir,
    shell_error::generic::GenericError,
};

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

    fn extra_description(&self) -> &str {
        "Use `--` before the command name to forward flags to the built-in command.\n\
         Flag strings, including those in spread lists, are resolved using the built-in's signature.\n\
         Use another `--` after the command name to pass subsequent flag strings literally."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Display help for a built-in command",
                example: "run-internal -- print -h",
                result: None,
            },
            Example {
                description: "Pass a named argument to a built-in command",
                example: "1.234 | run-internal -- 'into string' --decimals 2",
                result: Some(Value::test_string("1.23")),
            },
            Example {
                description: "Pass a flag string literally",
                example: "run-internal -- print -- -h",
                result: None,
            },
        ]
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
                "Arguments and flags to pass to the built-in command.",
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
        // Resolve flags before allocating any temporary argument slots, so errors during
        // argument binding cannot leak a partially built call frame onto the stack.
        let rest_args: Vec<(Value, bool)> = call.rest_preserving_spreads(engine_state, stack, 1)?;
        let args = bind_arguments(&decl.signature(), rest_args)?;

        // Help is normally rewritten by the compiler. Dynamic dispatch must render it
        // here, since calling Command::run directly bypasses that rewrite.
        if args
            .iter()
            .any(|arg| matches!(arg, Argument::Flag { data, name, .. } if &data[*name] == b"help"))
        {
            return Ok(
                Value::string(get_full_help(decl, engine_state, stack, head), head)
                    .into_pipeline_data(),
            );
        }

        // Build an IR call frame for the target builtin, preserving spread arguments.
        let mut builder = ir::Call::build(decl_id, head);
        for arg in args {
            builder.add_argument(stack, arg);
        }

        // `builder.with` is a scoped guard: it registers temporary IR argument slots,
        // calls the closure, then always deallocates those slots on exit.
        builder.with(stack, |stack, engine_call| {
            decl.run(engine_state, stack, engine_call, input)
        })
    }
}

/// Bind evaluated flag strings without reparsing positional values or losing rest spreads.
fn bind_arguments(
    signature: &Signature,
    rest_args: Vec<(Value, bool)>,
) -> Result<Vec<Argument>, ShellError> {
    let mut values = Vec::new();
    for (val, spread) in rest_args {
        if spread {
            match val {
                Value::List { vals, .. } => {
                    values.extend(vals.into_owned().into_iter().map(|val| (val, true)));
                }
                // Empty spreads must not interfere with no-argument defaults such as `ls`.
                Value::Nothing { .. } => {}
                Value::Error { error, .. } => return Err(*error),
                other => {
                    return Err(ShellError::CannotSpreadAsList { span: other.span() });
                }
            }
        } else {
            values.push((val, false));
        }
    }

    let mut values = values.into_iter();
    let mut args = Vec::new();
    let mut literal = false;
    while let Some((val, spread)) = values.next() {
        let span = val.span();
        if !literal && let Value::String { val: text, .. } = &val {
            if text == "--" {
                literal = true;
                continue;
            }

            let (flag_text, inline) = text
                .split_once('=')
                .map_or((text.as_str(), None), |(flag, value)| (flag, Some(value)));
            let unknown_flag = || {
                ShellError::Generic(GenericError::new(
                    format!("Unknown flag `{flag_text}`"),
                    format!(
                        "Use `run-internal -- '{}' --help` to see available flags",
                        signature.name
                    ),
                    span,
                ))
            };
            let flags = if let Some(long) = flag_text.strip_prefix("--") {
                vec![signature.get_long_flag(long).ok_or_else(unknown_flag)?]
            } else if let Some(shorts) = flag_text.strip_prefix('-')
                && !shorts.is_empty()
                && flag_text.parse::<f64>().is_err()
            {
                shorts
                    .chars()
                    .map(|short| signature.get_short_flag(short).ok_or_else(unknown_flag))
                    .collect::<Result<Vec<_>, _>>()?
            } else {
                Vec::new()
            };

            let count = flags.len();
            for (index, flag) in flags.into_iter().enumerate() {
                if flag.arg.is_some() && index + 1 < count {
                    return Err(ShellError::Generic(GenericError::new(
                        "Only the last flag in a batch can take a value",
                        format!("Pass `--{}` separately", flag.long),
                        span,
                    )));
                }
                let value = if index + 1 == count
                    && let Some(inline) = inline
                {
                    Value::string(inline, span)
                } else if flag.arg.is_some() {
                    values.next().map(|(val, _)| val).ok_or_else(|| {
                        ShellError::MissingParameter {
                            param_name: format!("value for --{}", flag.long),
                            span,
                        }
                    })?
                } else {
                    Value::bool(true, span)
                };

                // Wrapped commands often supply strings. Parse only literal flag values
                // whose type does not already match; never evaluate strings as Nu code.
                let expected = flag.arg.as_ref().map_or(Type::Bool, SyntaxShape::to_type);
                let value = if let Value::String { val: text, .. } = &value
                    && !value.is_assignable_to(&expected)
                {
                    nuon::from_nuon(text, Some(value.span()))?.with_span(value.span())
                } else {
                    value
                };
                let mut record = Record::new();
                record.push(flag.long, value);
                // Reuse the engine's signature validation, switch and null semantics.
                args.extend(expand_flag_record(signature, record, span)?);
            }
            if count > 0 {
                continue;
            }
        }

        if spread {
            // Reassemble contiguous positional spread values so they still fill rest
            // arguments instead of accidentally binding required/optional positionals.
            if let Some(Argument::Spread {
                span: spread_span,
                vals: Value::List { vals, .. },
                ..
            }) = args.last_mut()
            {
                *spread_span = spread_span.merge(span);
                vals.to_mut().push(val);
            } else {
                args.push(Argument::Spread {
                    span,
                    vals: Value::list(vec![val], span),
                    ast: None,
                });
            }
        } else {
            args.push(Argument::Positional {
                span,
                val,
                ast: None,
            });
        }
    }
    Ok(args)
}
