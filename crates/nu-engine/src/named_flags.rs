//! Helpers for signature-aware named flag handling (null omit/pass-through and
//! long-key record flag spreads).
//!
//! **IR-first:** Normal script evaluation uses IR (`normalize_engine_arguments` before
//! custom/builtin dispatch). The light AST custom-command path reuses these helpers for
//! parity; builtins on the residual AST `eval_call` path rely on IR for everyday use.

use std::sync::Arc;

use nu_protocol::{
    CompareTypes, Flag, Record, ShellError, Signature, Span, Value, engine::Argument,
    ir::DataSlice, shell_error::generic::GenericError,
};

/// Whether a named flag's value type accepts `nothing`/`null`.
///
/// Thin wrapper around [`Flag::type_accepts_nothing`] for local call sites.
#[inline]
pub(crate) fn flag_type_accepts_nothing(flag: &Flag) -> bool {
    flag.type_accepts_nothing()
}

pub(crate) fn find_signature_flag<'a>(
    signature: &'a Signature,
    long: &[u8],
    short: &[u8],
) -> Option<&'a Flag> {
    signature.named.iter().find(|flag| {
        (!long.is_empty() && flag.long.as_bytes() == long)
            || (!short.is_empty()
                && flag.short.is_some_and(|c| {
                    let mut buf = [0u8; 4];
                    c.encode_utf8(&mut buf).as_bytes() == short
                }))
    })
}

pub(crate) fn data_from_name_and_short(
    name: &str,
    short: &str,
) -> (Arc<[u8]>, DataSlice, DataSlice) {
    let data: Vec<u8> = name.bytes().chain(short.bytes()).collect();
    let data: Arc<[u8]> = data.into();
    // Flag names are parser identifiers — never near u32::MAX.
    #[allow(clippy::cast_possible_truncation)]
    let name_len = name.len() as u32;
    #[allow(clippy::cast_possible_truncation)]
    let short_len = short.len() as u32;
    let name = DataSlice {
        start: 0,
        len: name_len,
    };
    let short = DataSlice {
        start: name_len,
        len: short_len,
    };
    (data, name, short)
}

/// Expand a record into named/flag arguments for a call.
///
/// - `null` field values: passed through if the flag type accepts `nothing`, otherwise omitted
/// - switch flags (`--flag` with no value): `true` sets the flag, `false`/`null` omit it
/// - valued flags: non-null values become `--name value` (type-checked against the signature)
pub(crate) fn expand_flag_record(
    signature: &Signature,
    record: Record,
    spread_span: Span,
) -> Result<Vec<Argument>, ShellError> {
    let mut out = Vec::with_capacity(record.len());
    for (key, val) in record {
        let Some(flag) = signature.get_long_flag(&key) else {
            return Err(ShellError::Generic(GenericError::new(
                format!("Unknown flag `{key}` in spread record"),
                format!("`{key}` is not a named argument of this command"),
                spread_span,
            )));
        };

        let short = flag
            .short
            .map(|c| {
                let mut buf = [0u8; 4];
                c.encode_utf8(&mut buf).to_string()
            })
            .unwrap_or_default();
        let (data, name_slice, short_slice) = data_from_name_and_short(&flag.long, &short);

        if flag.arg.is_none() {
            // Switch: only `true` sets the flag; `false`/`null` omit (like `--flag=false`).
            match val {
                Value::Bool { val: true, .. } => {
                    out.push(Argument::Flag {
                        data,
                        name: name_slice,
                        short: short_slice,
                        span: spread_span,
                    });
                }
                Value::Bool { val: false, .. } | Value::Nothing { .. } => {}
                other => {
                    return Err(ShellError::CantConvert {
                        to_type: "bool".into(),
                        from_type: other.get_type().to_string(),
                        span: other.span(),
                        help: Some(format!(
                            "spread field `{key}` is a switch; use true/false or omit/null"
                        )),
                    });
                }
            }
        } else if val.is_nothing() && !flag_type_accepts_nothing(&flag) {
            // Null → omit when the flag type does not accept nothing.
        } else {
            if !val.is_nothing()
                && let Some(shape) = &flag.arg
            {
                let expected = shape.to_type();
                if !val.is_assignable_to(&expected) {
                    return Err(ShellError::CantConvert {
                        to_type: expected.to_string(),
                        from_type: val.get_type().to_string(),
                        span: val.span(),
                        help: Some(format!(
                            "spread field `{key}` does not match the type of `--{key}`"
                        )),
                    });
                }
            }
            out.push(Argument::Named {
                data,
                name: name_slice,
                short: short_slice,
                span: spread_span,
                val,
                ast: None,
            });
        }
    }
    Ok(out)
}

/// Whether this signature can accept a list rest spread.
pub(crate) fn can_rest_spread(signature: &Signature) -> bool {
    signature.rest_positional.is_some() || signature.allows_unknown_args
}

/// Error when a list (or null rest-mode) is spread while required positionals remain unbound.
///
/// Dual-purpose commands accept both flag records and rest lists via `...$x`. Spreading a list
/// before required positionals are filled would leave them unbound (list items go to rest).
pub(crate) fn list_spread_before_required_error(spread_span: Span) -> ShellError {
    ShellError::Generic(GenericError::new(
        "Cannot spread a list before required positional arguments are provided",
        "List spreads fill rest arguments. Provide required positionals first, or use a record to spread named flags, e.g. ...{flag: value}",
        spread_span,
    ))
}

/// Apply signature-aware null handling and expand record spreads for engine [`Argument`]s.
///
/// List spreads are rejected when the command has no rest parameter (and does not allow
/// unknown args), so `...$list` cannot be silently dropped on named-only commands.
pub(crate) fn normalize_engine_arguments(
    signature: &Signature,
    args: Vec<Argument>,
) -> Result<Vec<Argument>, ShellError> {
    let mut expanded = Vec::with_capacity(args.len());

    for arg in args {
        match arg {
            Argument::Named {
                data,
                name,
                short,
                span,
                val: Value::Nothing { .. },
                ast,
            } => {
                let accepts = find_signature_flag(signature, &data[name], &data[short])
                    .is_some_and(flag_type_accepts_nothing);
                if accepts {
                    expanded.push(Argument::Named {
                        data,
                        name,
                        short,
                        span,
                        val: Value::nothing(span),
                        ast,
                    });
                }
                // else: null → omit
            }
            Argument::Spread {
                vals,
                span: spread_span,
                ast,
            } => match vals {
                Value::Record { val, .. } => {
                    expanded.extend(expand_flag_record(
                        signature,
                        val.into_owned(),
                        spread_span,
                    )?);
                }
                Value::List { .. } => {
                    if !can_rest_spread(signature) {
                        return Err(ShellError::Generic(GenericError::new(
                            "Cannot spread a list into this command",
                            "This command has no ...rest parameter to receive a list spread. Use a record to spread named flags, e.g. ...{flag: value}",
                            spread_span,
                        )));
                    }
                    expanded.push(Argument::Spread {
                        vals,
                        span: spread_span,
                        ast,
                    });
                }
                Value::Nothing { .. } | Value::Error { .. } => {
                    expanded.push(Argument::Spread {
                        vals,
                        span: spread_span,
                        ast,
                    });
                }
                other => {
                    return Err(ShellError::CannotSpreadAsList { span: other.span() });
                }
            },
            other => expanded.push(other),
        }
    }

    Ok(expanded)
}
