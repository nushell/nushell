//! Helpers for signature-aware named flag handling (null omit/pass-through and
//! record flag spreads). Shared by IR and AST evaluation paths.

use std::sync::Arc;

use nu_protocol::{
    CompareTypes, Flag, Record, ShellError, Signature, Span, Value, engine::Argument,
    ir::DataSlice, shell_error::generic::GenericError,
};

/// Whether a named flag's value type accepts `nothing`/`null`.
///
/// Thin wrapper around [`Flag::type_accepts_nothing`] for call sites that already
/// have a [`Flag`] reference.
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

/// Build the packed long+short name buffer used by engine [`Argument`]s.
///
/// Flag names are always short identifiers from the parser; lengths always fit in `u32`.
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

/// Decode long/short flag name slices packed by [`data_from_name_and_short`].
pub(crate) fn long_short_from_data(
    data: &[u8],
    name: DataSlice,
    short: DataSlice,
    span: Span,
) -> Result<(&str, Option<String>), ShellError> {
    let long = std::str::from_utf8(&data[name]).map_err(|_| {
        ShellError::Generic(GenericError::new(
            "Invalid flag name encoding",
            "flag long name is not valid UTF-8",
            span,
        ))
    })?;
    let short_str = std::str::from_utf8(&data[short]).map_err(|_| {
        ShellError::Generic(GenericError::new(
            "Invalid flag name encoding",
            "flag short name is not valid UTF-8",
            span,
        ))
    })?;
    Ok((long, (!short_str.is_empty()).then(|| short_str.to_string())))
}

/// Resolve a record key to a signature flag: long name first, then single-char short.
/// Long wins when both a long flag named `a` and a short `-a` exist.
fn flag_for_spread_key(signature: &Signature, key: &str) -> Option<Flag> {
    signature.get_long_flag(key).or_else(|| {
        // Single Unicode scalar → short flag (e.g. key "a" → `-a`, including short-only flags).
        let mut chars = key.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => signature.get_short_flag(c),
            _ => None,
        }
    })
}

fn flag_cli_name(flag: &Flag) -> String {
    if let Some(long) = flag.long_name() {
        format!("--{long}")
    } else if let Some(short) = flag.short {
        format!("-{short}")
    } else {
        "<flag>".into()
    }
}

/// Expand a record into named/flag arguments for a call.
///
/// - Record keys match **long** flag names first; a single-character key may also match a
///   **short** flag (including short-only flags). Long form wins on collision.
/// - Unknown keys: error, unless [`Signature::allows_unknown_args`] is set (then passed through
///   as positionals / rest tokens, matching CLI unknown-flag handling).
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
        let Some(flag) = flag_for_spread_key(signature, &key) else {
            if signature.allows_unknown_args {
                push_unknown_spread_field(&mut out, &key, val, spread_span);
                continue;
            }
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
        } else if val.is_nothing() && !flag.type_accepts_nothing() {
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
                            "spread field `{key}` does not match the type of `{}`",
                            flag_cli_name(&flag)
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

/// Forward an unknown record field when `allows_unknown_args` is set.
///
/// Emits CLI-like rest/unknown tokens as [`Argument::Positional`] (`--key` / `-k` plus optional
/// value), matching how the parser turns unknown flags into `Unknown` args (which compile to
/// positionals). Custom commands bind these into remaining positionals / `...rest` in order —
/// the same as `f --extra hello` on a `--wrapped` command. We do not emit [`Argument::Named`] /
/// [`Argument::Flag`]: custom commands only bind declared named params by `var_id`.
fn push_unknown_spread_field(out: &mut Vec<Argument>, key: &str, val: Value, spread_span: Span) {
    let flag_token = if key.chars().count() == 1 {
        format!("-{key}")
    } else {
        format!("--{key}")
    };

    match val {
        Value::Bool { val: true, .. } => {
            out.push(Argument::Positional {
                span: spread_span,
                val: Value::string(flag_token, spread_span),
                ast: None,
            });
        }
        Value::Bool { val: false, .. } | Value::Nothing { .. } => {}
        other => {
            out.push(Argument::Positional {
                span: spread_span,
                val: Value::string(flag_token, spread_span),
                ast: None,
            });
            out.push(Argument::Positional {
                span: other.span(),
                val: other,
                ast: None,
            });
        }
    }
}

/// Whether this signature can accept a list rest spread.
pub(crate) fn can_rest_spread(signature: &Signature) -> bool {
    signature.rest_positional.is_some() || signature.allows_unknown_args
}

/// Error when a list is spread while required positionals remain unbound.
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
                val: val @ Value::Nothing { .. },
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
                        val,
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
