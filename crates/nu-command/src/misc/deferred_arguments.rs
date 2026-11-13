//! Binding arguments that the parser read without the callee's signature.
//!
//! The parser can't see the signature of a script's `main` when it parses `run <script> ...args`,
//! or of the builtin `%$cmd ...args` names, because that one is only known at runtime. It parses
//! those arguments as `any` instead (`add_script_arguments` in `nu-parser`), and `run` and
//! `run-internal` bind them to the callee's signature here, when they execute.

use nu_engine::{command_prelude::*, get_eval_expression};
use nu_protocol::{
    Flag, ParseError, ast, did_you_mean,
    engine::{
        self, CallImpl,
        named_flags::{
            can_rest_spread, data_from_name_and_short, find_signature_flag,
            list_spread_before_required_error, normalize_engine_arguments,
        },
    },
};
use std::cell::Cell;

/// An argument the parser read without the callee's signature, evaluated and ready to bind.
///
/// Each one keeps the span of the argument in the call, so errors point at it and the binder can
/// read its source text.
pub(crate) enum DeferredArgument {
    Positional(Value, Span),
    Spread(Value, Span),
    /// A `--name=value` that the parser split off.
    Named {
        long: String,
        value: Value,
        span: Span,
    },
}

impl DeferredArgument {
    pub(crate) fn span(&self) -> Span {
        match self {
            DeferredArgument::Positional(_, span)
            | DeferredArgument::Spread(_, span)
            | DeferredArgument::Named { span, .. } => *span,
        }
    }
}

/// Where [`bind_deferred_arguments`] puts the arguments it binds.
pub(crate) trait ArgumentTarget {
    /// Add the next positional argument. The signature has a parameter for it.
    fn add_positional(
        &mut self,
        signature: &Signature,
        value: Value,
        span: Span,
    ) -> Result<(), ShellError>;

    /// Spread `value` into the call: a record sets flags, and a list or `null` fills the rest
    /// parameter.
    fn add_spread(
        &mut self,
        signature: &Signature,
        value: Value,
        span: Span,
    ) -> Result<(), ShellError>;

    /// Set `flag`, written at `span`, with its value and the value's span, or with no value for a
    /// switch written on its own. A `null` value is one the flag's type accepts.
    fn add_named(
        &mut self,
        signature: &Signature,
        flag: &Flag,
        span: Span,
        value: Option<(Value, Span)>,
    ) -> Result<(), ShellError>;
}

/// The arguments of `call` to `command`, as the IR evaluator passes them to it.
///
/// A call that wasn't compiled has its arguments evaluated here and checked against `command`'s
/// signature, as the IR evaluator would: record spreads expand into its flags, and a `null` value
/// leaves out a flag whose type doesn't take it.
pub(crate) fn call_arguments(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    command: &dyn Command,
) -> Result<Vec<engine::Argument>, ShellError> {
    match &call.inner {
        CallImpl::IrRef(call) => Ok(call.arguments(stack).to_vec()),
        CallImpl::IrBox(call) => Ok(call.arguments(stack).to_vec()),
        CallImpl::AstRef(call) => ast_call_arguments(engine_state, stack, call, command),
        CallImpl::AstBox(call) => ast_call_arguments(engine_state, stack, call, command),
    }
}

/// Evaluate the arguments of a call that wasn't compiled, as the IR evaluator would pass them.
fn ast_call_arguments(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &ast::Call,
    command: &dyn Command,
) -> Result<Vec<engine::Argument>, ShellError> {
    let stack = &mut stack.use_call_arg_out_dest();
    let eval_expression = get_eval_expression(engine_state);
    let args = call
        .arguments
        .iter()
        .map(|arg| {
            let span = arg.span();
            Ok(match arg {
                ast::Argument::Positional(expr) | ast::Argument::Unknown(expr) => {
                    engine::Argument::Positional {
                        span,
                        val: eval_expression(engine_state, stack, expr)?,
                    }
                }
                ast::Argument::Spread(expr) => engine::Argument::Spread {
                    span,
                    vals: eval_expression(engine_state, stack, expr)?,
                },
                ast::Argument::Named((long, short, expr)) => {
                    let short = short.as_ref().map_or("", |short| short.item.as_str());
                    let (data, name, short) = data_from_name_and_short(&long.item, short);
                    match expr {
                        Some(expr) => engine::Argument::Named {
                            data,
                            name,
                            short,
                            span,
                            val: eval_expression(engine_state, stack, expr)?,
                        },
                        None => engine::Argument::Flag {
                            data,
                            name,
                            short,
                            span,
                        },
                    }
                }
            })
        })
        .collect::<Result<_, ShellError>>()?;
    normalize_engine_arguments(&command.signature(), args)
}

/// Split off the callee's arguments: the ones after the first positional argument, which is the
/// script path of `run` or the builtin name of `run-internal`. The command's own arguments, up to
/// and including that one, stay in `args`.
pub(crate) fn split_off_callee_arguments(
    args: &mut Vec<engine::Argument>,
) -> Vec<engine::Argument> {
    let first = args
        .iter()
        .position(|arg| matches!(arg, engine::Argument::Positional { .. }))
        .map_or(args.len(), |position| position + 1);
    args.split_off(first)
}

/// The callee's arguments from [`split_off_callee_arguments`], as the parser deferred them.
///
/// A named argument among them is a `--name=value` the parser split off.
pub(crate) fn deferred_arguments(
    args: Vec<engine::Argument>,
) -> Result<Vec<DeferredArgument>, ShellError> {
    args.into_iter()
        .filter_map(|arg| match arg {
            engine::Argument::Positional { val, span } => {
                Some(Ok(DeferredArgument::Positional(val, span)))
            }
            engine::Argument::Spread { vals, span } => Some(
                unwrap_deferred_value(vals, span)
                    .map(|value| DeferredArgument::Spread(value, span)),
            ),
            engine::Argument::Named {
                data,
                name,
                span,
                val,
                ..
            } => Some(
                unwrap_deferred_value(val, span).map(|value| DeferredArgument::Named {
                    long: String::from_utf8_lossy(&data[name]).into_owned(),
                    value,
                    span,
                }),
            ),
            // The parser adds no flag without a value to deferred arguments.
            engine::Argument::Flag { .. } | engine::Argument::ParserInfo { .. } => None,
        })
        .collect()
}

/// The value of a spread or a `--name=value`, which the parser wraps in a one-item list.
fn unwrap_deferred_value(wrapped: Value, span: Span) -> Result<Value, ShellError> {
    let item = match wrapped {
        Value::List { vals, .. } => <[Value; 1]>::try_from(vals.into_owned()).ok(),
        _ => None,
    };
    item.map(|[value]| value)
        .ok_or_else(|| ShellError::NushellFailedSpanned {
            msg: "expected the parser to wrap a spread or `--name=value` argument in a list".into(),
            label: "this argument".into(),
            span,
        })
}

/// A bare argument whose source text is a flag or `--`.
enum FlagToken<'a> {
    /// `--`: every argument after it is positional.
    EndOfOptions,
    /// `--name`, with the text after the `=` of a `--name=value` the parser left a word.
    Long(&'a str, Option<&'a str>),
    /// A batch of short flags like `-abc`: the flags before the last one, and the last one, which
    /// can take a value.
    Short(Vec<&'a Flag>, &'a Flag),
    /// A batch of short flags with a character that isn't a short flag of the callee.
    UnknownShort(char),
}

/// Recognize a flag from the source text of a deferred argument, as the parser does.
///
/// Only a bare word can be a flag, so a quoted `"-c"` or a variable holding `"--name"` stays a
/// value. `-abc` is a batch of short flags, and a character in it that isn't a short flag of the
/// callee makes it an unknown flag, so a mistyped `-dry-run` is an error rather than a value. A
/// word that names no short flag of the callee stays a value when the parser read it as a number,
/// filesize, duration or range, like `-1`, `-inf`, `-5kb` or `-1..5`. The parser read it without
/// the callee's signature, so this is looser than a direct call, which takes only numbers, and
/// only where the parameter takes a number.
fn flag_token<'a>(
    engine_state: &'a EngineState,
    signature: &'a Signature,
    value: &Value,
    span: Span,
) -> Option<FlagToken<'a>> {
    let token = std::str::from_utf8(engine_state.get_span_contents(span)).ok()?;
    if token == "--" {
        return Some(FlagToken::EndOfOptions);
    }

    if let Some(flag) = token.strip_prefix("--") {
        // The parser splits `--name=value` off, except `--help=...`, `--=...`, and one after a
        // `--` that a flag turns out to take as its value. That value is only text.
        return Some(match flag.split_once('=') {
            Some((name, value)) => FlagToken::Long(name, Some(value)),
            None => FlagToken::Long(flag, None),
        });
    }

    let shorts = token
        .strip_prefix('-')
        .filter(|shorts| !shorts.is_empty())?;
    let mut flags = Vec::new();
    let mut unknown = None;
    for short in shorts.chars() {
        match signature
            .named
            .iter()
            .find(|flag| flag.short == Some(short))
        {
            Some(flag) => flags.push(flag),
            None => {
                unknown.get_or_insert(short);
            }
        }
    }
    if flags.is_empty()
        && matches!(
            value,
            Value::Int { .. }
                | Value::Float { .. }
                | Value::Filesize { .. }
                | Value::Duration { .. }
                | Value::Range { .. }
        )
    {
        return None;
    }
    Some(match unknown {
        Some(short) => FlagToken::UnknownShort(short),
        None => {
            let last = flags.pop()?;
            FlagToken::Short(flags, last)
        }
    })
}

/// Bind deferred arguments to the callee's `signature`, putting them in `target`.
///
/// The parser parsed each argument as `any`, without the callee's signature, splitting
/// `--name=value` into a named argument with a parsed value. Here a bare word is a flag when its
/// source text is one, as for the parser, so a quoted `"-c"`, a variable holding `"--name"` and the
/// items of a spread list stay values, while a spread record sets flags. What a direct call rejects
/// is an error here too: unknown flags, flags missing their value and extra positional arguments.
/// A list spread before a required positional argument is one, as for a custom command. The target
/// checks the types of the values.
///
/// Returns `true` when the arguments ask for the callee's help (`--help`, `-h` or
/// `...{help: true}`). As in a direct call, the other arguments are checked first.
pub(crate) fn bind_deferred_arguments<T: ArgumentTarget>(
    engine_state: &EngineState,
    signature: &Signature,
    args: Vec<DeferredArgument>,
    target: &mut T,
) -> Result<bool, ShellError> {
    let find_long_flag = |name: &str| find_signature_flag(signature, name.as_bytes(), b"");
    let positional_count = Cell::new(0);
    let add_positional = |target: &mut T, value: Value, span: Span| -> Result<(), ShellError> {
        if signature.get_positional(positional_count.get()).is_none() {
            return Err(ShellError::from_diagnostic(&ParseError::ExtraPositional(
                signature.call_signature(),
                span,
            )));
        }
        positional_count.set(positional_count.get() + 1);
        target.add_positional(signature, value, span)
    };
    let mut help = false;
    let mut end_of_options = false;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        let span = arg.span();
        // The flag the argument names, or its name for an error, and the value written with it
        // (`--name=value`) with the span of that value.
        let (flag, inline_value) = match arg {
            DeferredArgument::Spread(mut value, _) => {
                // `...{help: true}` asks for the callee's help, like `--help`, when it has one.
                if let Value::Record { val, .. } = &mut value
                    && matches!(val.get("help"), Some(Value::Bool { val: true, .. }))
                    && find_long_flag("help").is_some()
                {
                    help = true;
                    val.to_mut().remove("help");
                }
                // A list fills the rest parameter, so it would leave a required positional
                // argument unbound.
                if matches!(value, Value::List { .. })
                    && positional_count.get() < signature.required_positional.len()
                {
                    return Err(list_spread_before_required_error(span));
                }
                // A record sets the callee's flags. `null` adds nothing to a callee without a rest
                // parameter, so the arguments after it still fill its positionals.
                if !value.is_nothing() || can_rest_spread(signature) {
                    target.add_spread(signature, value, span)?;
                }
                continue;
            }
            // `--name=value`, which the parser splits off only before a `--`.
            DeferredArgument::Named { long, value, .. } => {
                let value_span = Span::new(span.start + long.len() + 3, span.end);
                (find_long_flag(&long).ok_or(long), Some((value, value_span)))
            }
            DeferredArgument::Positional(value, _) => {
                let token = if end_of_options {
                    None
                } else {
                    flag_token(engine_state, signature, &value, span)
                };
                match token {
                    None => {
                        add_positional(target, value, span)?;
                        continue;
                    }
                    Some(FlagToken::EndOfOptions) => {
                        end_of_options = true;
                        // A wrapped callee gets the `--` too, like a direct call to it does.
                        if signature.allows_unknown_args {
                            add_positional(target, value, span)?;
                        }
                        continue;
                    }
                    Some(FlagToken::Long(name, inline)) => (
                        find_long_flag(name).ok_or_else(|| name.to_owned()),
                        inline.map(|text| {
                            let value_span = Span::new(span.start + name.len() + 3, span.end);
                            (Value::string(text, value_span), value_span)
                        }),
                    ),
                    Some(FlagToken::UnknownShort(short)) => (Err(format!("-{short}")), None),
                    Some(FlagToken::Short(leading, last)) => {
                        // Like a direct call, `-abc` sets each flag it names, and only the last
                        // one can take a value.
                        for flag in leading {
                            if flag.arg.is_some() {
                                return Err(ShellError::from_diagnostic(
                                    &ParseError::OnlyLastFlagInBatchCanTakeArg(span),
                                ));
                            }
                            if flag.long == "help" {
                                help = true;
                            } else {
                                target.add_named(signature, flag, span, None)?;
                            }
                        }
                        (Ok(last), None)
                    }
                }
            }
        };

        let flag = match flag {
            Ok(flag) => flag,
            // A wrapped callee takes an unknown flag as an argument, as it's written.
            Err(_) if signature.allows_unknown_args => {
                let text = String::from_utf8_lossy(engine_state.get_span_contents(span));
                add_positional(target, Value::string(text, span), span)?;
                continue;
            }
            Err(name) => {
                // Like the parser, only suggest a name for a long flag.
                let help = if name.starts_with('-') {
                    None
                } else {
                    did_you_mean(signature.get_names(), &name)
                        .map(|name| format!("Did you mean: `--{name}`?"))
                };
                return Err(ShellError::from_diagnostic(&ParseError::UnknownFlag(
                    signature.name.clone(),
                    name,
                    span,
                    help.unwrap_or_else(|| "Use `--help` to see available flags".to_owned()),
                )));
            }
        };

        // Like a direct call, `--help` asks for the callee's help, even when it declares the flag.
        if flag.long == "help" {
            help = true;
            continue;
        }

        let value = match (&flag.arg, inline_value) {
            // `--name=value`, or `--switch=true` and `--switch=false`.
            (_, Some(value)) => Some(value),
            (None, None) => None,
            // The flag's value is the next argument, as in a direct call.
            (Some(shape), None) => match args.next() {
                Some(DeferredArgument::Positional(value, value_span)) => Some((value, value_span)),
                // A `--name=value` or a spread is the value as it's written. The span of a spread
                // starts after its `...`.
                Some(arg) => {
                    let value_span = arg.span();
                    let text = String::from_utf8_lossy(engine_state.get_span_contents(value_span));
                    let text = if matches!(arg, DeferredArgument::Spread(..)) {
                        format!("...{text}")
                    } else {
                        text.into_owned()
                    };
                    Some((Value::string(text, value_span), value_span))
                }
                None => {
                    return Err(ShellError::from_diagnostic(&ParseError::MissingFlagParam(
                        shape.to_string(),
                        span,
                    )));
                }
            },
        };

        // `null` leaves out a flag whose type doesn't take it, as a `null` variable does in a
        // direct call. (The parser rejects a written `null` there.)
        if value
            .as_ref()
            .is_some_and(|(value, _)| value.is_nothing() && !flag.type_accepts_nothing())
        {
            continue;
        }

        target.add_named(signature, flag, span, value)?;
    }

    Ok(help)
}
