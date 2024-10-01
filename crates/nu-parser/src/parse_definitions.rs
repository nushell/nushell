//! # Parsing of Definitions
//!
//! Definitions include `def` and `extern`. Parsing of definitions
//! include 3 steps:
//!
//!     1. Prepare the definition from spans: Ensure the definition is
//!     well formed, erroring out otherwise
//!
//!     2. Declaring the definition: Since definitions can be used before
//!     they are used in a scope (including their own bodies in the case
//!     of recursion), we need to make them visible before parsing their
//!     bodies.
//!
//!     3. Actually parsing the definition: Now we can actually parse
//!     the definition.
//!
//! Step 2 and 3, reuse the [Definition] created in step one and avoid
//! the past problem of parsing definitions twice.
use crate::{
    lex, lex_signature, parse_block,
    parse_shape_specs::parse_type,
    parser::{garbage_pipeline, parse_signature, parse_string},
    type_check::check_block_input_output,
    LiteCommand, RESERVED_VARIABLE_NAMES,
};
use itertools::Itertools;
use nu_protocol::{
    ast::{Call, Expr, Expression, Pipeline},
    engine::StateWorkingSet,
    DeclId, ParseError, Signature, Span, Spanned, Type,
};
use std::sync::Arc;

/// Ensure the definition is well formed, erroring out otherwise
pub fn prepare_definition<'lite_command>(
    working_set: &mut StateWorkingSet,
    lite_command: &'lite_command LiteCommand,
) -> Option<Definition<'lite_command>> {
    let full_span = Span::concat(&lite_command.parts);
    let comments = lite_command.comments.as_slice();
    let mut spans_iter = lite_command.parts.iter();

    let maybe_export_span = spans_iter.next()?;
    let maybe_export_bytes = working_set.get_span_contents(*maybe_export_span);

    let (kw_span, export_span) = if matches!(maybe_export_bytes, b"export") {
        let kw_span = spans_iter.next()?;
        (kw_span, Some(maybe_export_span))
    } else {
        (maybe_export_span, None)
    };

    let kw_bytes = working_set.get_span_contents(*kw_span);
    let kind = DefKind::try_from_bytes(kw_bytes)?;

    let mut flag_spans = Vec::new();
    let Some(name_span) = spans_iter
        .find(|span| !added_flag(working_set, &mut flag_spans, **span))
        .copied()
    else {
        let span = {
            let point = flag_spans.last().map(|it| it.end).unwrap_or(kw_span.end);

            Span::new(point, point)
        };

        working_set.error(DefError::MissingName.into_error(kind, span));
        return None;
    };

    if let Some(err) = detect_params_in_name(working_set, name_span, kind.as_str()) {
        working_set.error(err);
        return None;
    }

    let name_str = parse_string(working_set, name_span).as_string()?;
    if name_str.contains('#')
        || name_str.contains('^')
        || name_str.parse::<bytesize::ByteSize>().is_ok()
        || name_str.parse::<f64>().is_ok()
    {
        working_set.error(DefError::InvalidName.into_error(kind, name_span));
        return None;
    }

    let Some(mut sig_span) = spans_iter
        .find(|span| !added_flag(working_set, &mut flag_spans, **span))
        .copied()
    else {
        let span = {
            let point = flag_spans.last().map(|it| it.end).unwrap_or(name_span.end);
            Span::new(point, point)
        };

        working_set.error(DefError::MissingSignature.into_error(kind, span));
        return None;
    };

    let sig_bytes = working_set.get_span_contents(sig_span);
    if !(sig_bytes.starts_with(b"[") || sig_bytes.starts_with(b"(")) {
        working_set.error(DefError::MissingSignature.into_error(kind, sig_span));
        return None;
    }

    let Some(next_span) = spans_iter
        .find(|span| !added_flag(working_set, &mut flag_spans, **span))
        .copied()
    else {
        return Some(Definition {
            comments,
            keyword: *kw_span,
            export: export_span.copied(),
            kind,
            name: name_span,
            signature: sig_span,
            input_output: None,
            block: None,
            flags: flag_spans,
            full: full_span,
        });
    };

    let next_bytes = working_set.get_span_contents(next_span);
    let (io_span, block_span) = if matches!(next_bytes, b":") {
        let Some(mut io_span) = spans_iter.next().copied() else {
            return Some(Definition {
                comments,
                keyword: *kw_span,
                export: export_span.copied(),
                kind,
                name: name_span,
                signature: sig_span,
                input_output: None,
                block: None,
                flags: flag_spans,
                full: full_span,
            });
        };

        let Some(block_span) = spans_iter
            .find(|span| {
                let bytes = working_set.get_span_contents(**span);
                if bytes.starts_with(b"{") {
                    true
                } else {
                    io_span = io_span.append(**span);

                    false
                }
            })
            .copied()
        else {
            return Some(Definition {
                comments,
                keyword: *kw_span,
                export: export_span.copied(),
                kind,
                name: name_span,
                signature: sig_span,
                input_output: Some(io_span),
                block: None,
                flags: flag_spans,
                full: full_span,
            });
        };

        (Some(io_span), Some(block_span))
    } else if sig_bytes.ends_with(b":") {
        sig_span.end -= 1;

        let mut io_span = next_span;
        let Some(block_span) = spans_iter
            .find(|span| {
                let bytes = working_set.get_span_contents(**span);
                if bytes.starts_with(b"{") {
                    true
                } else {
                    io_span = io_span.append(**span);

                    false
                }
            })
            .copied()
        else {
            return Some(Definition {
                comments,
                keyword: *kw_span,
                export: export_span.copied(),
                kind,
                name: name_span,
                signature: sig_span,
                input_output: Some(io_span),
                block: None,
                flags: flag_spans,
                full: full_span,
            });
        };

        (Some(io_span), Some(block_span))
    } else if next_bytes.starts_with(b"{") {
        (None, Some(next_span))
    } else {
        let mut erroneous_span = next_span;
        let Some(block_span) = spans_iter
            .find(|span| {
                let bytes = working_set.get_span_contents(**span);
                if bytes.starts_with(b"{") {
                    true
                } else {
                    erroneous_span = erroneous_span.append(**span);

                    false
                }
            })
            .copied()
        else {
            working_set.error(DefError::ExtraTokens.into_error(kind, erroneous_span));
            return None;
        };

        (None, Some(block_span))
    };

    let extra_span = spans_iter.filter(|span| !added_flag(working_set, &mut flag_spans, **span));
    let errors = extra_span
        .map(|span| DefError::ExtraTokens.into_error(kind, *span))
        .collect_vec();

    working_set.parse_errors.extend(errors);

    let def = Definition {
        comments,
        keyword: *kw_span,
        export: export_span.copied(),
        kind,
        name: name_span,
        signature: sig_span,
        input_output: io_span,
        block: block_span,
        flags: flag_spans,
        full: full_span,
    };

    Some(def)
}

/// Since definitions can be used before they are used in a scope
/// (including their own bodies in the case of recursion), we need
/// to make them visible before parsing their bodies.
pub fn declare_definition(
    working_set: &mut StateWorkingSet,
    definition: &Definition<'_>,
) -> Box<Signature> {
    let Definition {
        name,
        signature: sig_span,
        input_output,
        flags,
        kind,
        ..
    } = definition;

    working_set.enter_scope();

    let mut sig = parse_signature(working_set, *sig_span);

    if let Some(input_output) = input_output.as_ref().copied() {
        let io_types = parse_input_output_types(working_set, *kind, input_output);

        if let Expression {
            expr: Expr::Signature(signature),
            span,
            ..
        } = &mut sig
        {
            signature.input_output_types = io_types;
            span.end = input_output.end;
        }
    }

    // working_set.exit_scope();

    let Some(mut signature) = sig.as_signature() else {
        unreachable!()
    };

    signature.name = {
        let name = working_set.get_span_contents(*name);
        String::from_utf8_lossy(name).to_string()
    };

    if flags.iter().any(|flag_span| {
        let flag_bytes = working_set.get_span_contents(*flag_span);
        matches!(flag_bytes, b"--wrapped")
    }) {
        signature.allows_unknown_args = true;
    }

    for arg in &signature.required_positional {
        verify_not_reserved_variable_name(working_set, &arg.name, *sig_span);
    }

    for arg in &signature.optional_positional {
        verify_not_reserved_variable_name(working_set, &arg.name, *sig_span);
    }

    if let Some(arg) = &signature.rest_positional {
        verify_not_reserved_variable_name(working_set, &arg.name, *sig_span);
    }

    for flag in &signature.get_names() {
        verify_not_reserved_variable_name(working_set, flag, *sig_span);
    }

    let decl = signature.clone().predeclare();

    if working_set.add_predecl(decl).is_some() {
        working_set.error(DefError::Duplicated.into_error(*kind, *name));
    }

    signature
}

/// Actually parse a definition
pub fn parse_definition(
    working_set: &mut StateWorkingSet,
    definition: &Definition<'_>,
    mut signature: Box<Signature>,
    module_name: Option<&[u8]>,
) -> (Pipeline, Option<(Vec<u8>, DeclId)>) {
    let Definition {
        comments,
        keyword,
        name,
        signature: sig_span,
        block,
        flags,
        kind,
        full,
        ..
    } = definition;


    let name_str = signature.name.clone();

    if let Some(module_name) = module_name {
        if name_str.as_bytes() == module_name {
            working_set.error(DefError::NamedAsModule(name_str).into_error(*kind, *name));

            return (garbage_pipeline(working_set, &[*full]), None);
        }
    }

    // working_set.enter_scope();

    let maybe_block = if let Some(block_span) = block {
        if matches!(kind, DefKind::Extern) {
            working_set.error(DefError::ExternWithBlock.into_error(*kind, *block_span));

            None
        } else {
            let span = {
                let start = block_span.start + 1;
                let end = if working_set.get_span_contents(*block_span).ends_with(b"}") {
                    block_span.end - 1
                } else {
                    block_span.end
                };

                Span::new(start, end)
            };

            if span.end == block_span.end {
                let span = Span::new(span.end, span.end);
                working_set.error(ParseError::Unclosed(String::from("}"), span));
            }

            let input = working_set.get_span_contents(span);
            let (tokens, maybe_lex_error) = lex(input, span.start, &[], &[], false);

            if let Some(err) = maybe_lex_error {
                working_set.error(err);
            }

            let block = parse_block(working_set, &tokens, span, false, false);
            Some(working_set.add_block(Arc::from(block)))
        }
    } else {
        None
    };

    let Some(decl_id) = working_set.find_decl(kind.as_bytes()) else {
        working_set.error(DefError::Undeclared.into_error(*kind, *full));
        return (garbage_pipeline(working_set, &[*full]), None);
    };

    // let mut maybe_help = None;
    let mut maybe_env = None;
    let mut maybe_wrapped = None;

    for flag_span in flags {
        let flag_bytes = working_set.get_span_contents(*flag_span);

        if let Some(flag_bytes) = flag_bytes.strip_prefix(b"--") {
            let make_flag = || -> Spanned<String> {
                let item = String::from_utf8_lossy(flag_bytes).to_string();
                Spanned {
                    item,
                    span: *flag_span,
                }
            };

            if matches!(flag_bytes, b"help") {
                // maybe_help = Some(make_flag())
            } else if matches!(kind, DefKind::Def) && matches!(flag_bytes, b"env") {
                maybe_env = Some(make_flag());
            } else if matches!(kind, DefKind::Def) && matches!(flag_bytes, b"wrapped") {
                maybe_wrapped = Some(make_flag());
            } else {
                let flag = String::from_utf8_lossy(flag_bytes).to_string();
                working_set.error(DefError::UnknownFlag(flag).into_error(*kind, *flag_span));
            }
        } else if flag_bytes == b"-h" {
            // let item = String::from_utf8_lossy(flag_bytes).to_string();
            // let flag = Spanned {
            //     item,
            //     span: *flag_span,
            // };
            //
            // maybe_help = Some(flag);
        }
    }

    let mut result = None;
    if let Some(block_id) = maybe_block {
        let Some(decl_id) = working_set.find_predecl(name_str.as_bytes()) else {
            working_set.error(DefError::Undeclared.into_error(*kind, *name));
            return (garbage_pipeline(working_set, &[*full]), None);
        };

        if maybe_wrapped.is_some() {
            *signature = signature.add_help();
            signature.allows_unknown_args = true;
        }

        let (desc, extra_desc) = working_set.build_desc(comments);
        signature.description = desc;
        signature.extra_description = extra_desc;

        let declaration = working_set.get_decl_mut(decl_id);
        *declaration = signature.clone().into_block_command(block_id);

        let block = working_set.get_block_mut(block_id);
        block.signature.clone_from(&signature);
        block.redirect_env = maybe_env.is_some();

        if block.signature.input_output_types.is_empty() {
            block
                .signature
                .input_output_types
                .push((Type::Any, Type::Any));
        }

        let block = working_set.get_block(block_id);
        let errors = check_block_input_output(working_set, block);
        working_set.parse_errors.extend_from_slice(&errors);

        result = Some((name_str.as_bytes().to_vec(), decl_id));
    } else if matches!(kind, DefKind::Def) {
        let point = sig_span.end;
        let span = Span::new(point, point);
        working_set.error(DefError::MissingBlock.into_error(*kind, span));
    }

    let mut call = Call::new(*keyword);

    call.decl_id = decl_id;
    call.head = *keyword;

    let _ = working_set.add_span(call.head);

    call.add_positional(Expression::new(
        working_set,
        Expr::String(name_str.to_string()),
        *name,
        Type::String,
    ));

    call.add_positional(Expression::new(
        working_set,
        Expr::Signature(signature),
        *sig_span,
        Type::Any,
    ));

    if let Some((block_id, block_span)) = maybe_block.zip(*block) {
        call.add_positional(Expression::new(
            working_set,
            Expr::Block(block_id),
            block_span,
            Type::Any,
        ));
    }

    working_set.exit_scope();
    working_set.merge_predecl(name_str.as_bytes());

    (
        Pipeline::from_vec(vec![Expression::new(
            working_set,
            Expr::Call(Box::from(call)),
            *full,
            Type::Any,
        )]),
        result,
    )
}

#[derive(Debug)]
/// A prepared definition.
///
/// This definition only contains spans instead of any syntactical
/// structures.
pub struct Definition<'lite_command> {
    comments: &'lite_command [Span],
    export: Option<Span>,
    keyword: Span,
    name: Span,
    signature: Span,
    input_output: Option<Span>,
    block: Option<Span>,
    flags: Vec<Span>,
    kind: DefKind,
    full: Span,
}

impl Definition<'_> {
    pub const fn is_def(&self) -> bool {
        matches!(self.kind, DefKind::Def)
    }

    pub const fn is_public(&self) -> bool {
        self.export.is_some()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DefKind {
    Def,
    Extern,
}

impl DefKind {
    const fn as_bytes(&self) -> &[u8] {
        self.as_str().as_bytes()
    }

    const fn as_str(&self) -> &str {
        match self {
            Self::Def => "def",
            Self::Extern => "extern",
        }
    }

    const fn try_from_bytes(bytes: &[u8]) -> Option<Self> {
        match bytes {
            b"def" => Some(Self::Def),
            b"extern" => Some(Self::Extern),
            _ => None,
        }
    }
}

fn parse_input_output_types(
    working_set: &mut StateWorkingSet,
    kind: DefKind,
    span: Span,
) -> Vec<(Type, Type)> {
    let bytes = working_set.get_span_contents(span);
    let is_list = bytes.starts_with(b"[");

    let (bytes, span) = if is_list {
        let span = Span::new(span.start + 1, span.end);
        (&bytes[1..], span)
    } else {
        (bytes, span)
    };

    let (bytes, span) = if bytes.ends_with(b"]") {
        let span = Span::new(span.start, span.end - 1);
        (&bytes[..(bytes.len() - 1)], span)
    } else if is_list {
        working_set.error(DefError::UnclosedList.into_error(kind, span));
        return Vec::new();
    } else {
        (bytes, span)
    };

    let (tokens, maybe_lex_error) = lex_signature(bytes, span.start, b"\n\r,", &[], true);

    if let Some(lex_error) = maybe_lex_error {
        working_set.error(lex_error);
    }

    let mut types = Vec::new();
    let mut index = 0;

    while index < tokens.len() {
        let first_token = &tokens[index];
        let first_span = first_token.span;
        let first_bytes = working_set.get_span_contents(first_span).to_vec();
        let input_type = match first_bytes.as_slice() {
            b"->" => {
                let span = Span::new(first_span.start, first_span.start);
                working_set.error(DefError::MissingInputType.into_error(kind, span));
                break;
            }

            bytes if bytes.starts_with(b"-") => {
                working_set.error(DefError::UnexpectedFlag.into_error(kind, first_span));
                break;
            }

            bytes => parse_type(working_set, bytes, first_span),
        };

        index += 1;

        let Some(second_token) = tokens.get(index) else {
            let span = Span::new(first_span.end, first_span.end);
            working_set.error(DefError::MissingArrow.into_error(kind, span));
            break;
        };
        let second_span = second_token.span;
        let second_bytes = working_set.get_span_contents(second_span);
        if !matches!(second_bytes, b"->") {
            let span = Span::new(second_span.end, second_span.end);
            working_set.error(DefError::MissingArrow.into_error(kind, span));
            break;
        }

        index += 1;

        let Some(third_token) = tokens.get(index) else {
            let span = Span::new(second_span.start, second_span.start);
            working_set.error(DefError::MissingOutputType.into_error(kind, span));
            break;
        };
        let third_span = third_token.span;
        let third_bytes = working_set.get_span_contents(third_span).to_vec();

        if third_bytes.starts_with(b"-") {
            working_set.error(DefError::UnexpectedFlag.into_error(kind, third_span));
            break;
        }

        let output_type = parse_type(working_set, &third_bytes, third_span);

        types.push((input_type, output_type));
        index += 1;
    }

    let extra_tokens = &tokens[index..];
    let maybe_extra_span = extra_tokens
        .first()
        .zip(extra_tokens.last())
        .map(|(first, last)| first.span.append(last.span));

    if let Some(extra_span) = maybe_extra_span {
        working_set.error(DefError::ExtraTokens.into_error(kind, extra_span));
    }

    types
}

fn added_flag(working_set: &StateWorkingSet, flag_spans: &mut Vec<Span>, span: Span) -> bool {
    let bytes = working_set.get_span_contents(span);

    if bytes
        .strip_prefix(b"--")
        .or_else(|| bytes.strip_prefix(b"-"))
        .is_some()
    {
        flag_spans.push(span);

        true
    } else {
        false
    }
}

/// If `name` is a keyword, emit an error.
fn verify_not_reserved_variable_name(working_set: &mut StateWorkingSet, name: &str, span: Span) {
    if RESERVED_VARIABLE_NAMES.contains(&name) {
        working_set.error(ParseError::NameIsBuiltinVar(name.to_string(), span))
    }
}

fn detect_params_in_name(
    working_set: &StateWorkingSet,
    name_span: Span,
    decl_name: &str,
) -> Option<ParseError> {
    let name = working_set.get_span_contents(name_span);

    let extract_span = |delim: u8| {
        // it is okay to unwrap because we know the slice contains the byte
        let (idx, _) = name
            .iter()
            .find_position(|c| **c == delim)
            .unwrap_or((name.len(), &b' '));
        let param_span = Span::new(name_span.start + idx - 1, name_span.start + idx - 1);
        let error = ParseError::LabeledErrorWithHelp{
            error: "no space between name and parameters".into(),
            label: "expected space".into(),
            help: format!("consider adding a space between the `{decl_name}` command's name and its parameters"),
            span: param_span,
            };
        Some(error)
    };

    if name.contains(&b'[') {
        extract_span(b'[')
    } else if name.contains(&b'(') {
        extract_span(b'(')
    } else {
        None
    }
}

enum DefError {
    Duplicated,
    Undeclared,
    ExtraTokens,
    InvalidName,
    MissingName,
    MissingArrow,
    MissingBlock,
    UnclosedList,
    UnexpectedFlag,
    ExternWithBlock,
    MissingInputType,
    MissingSignature,
    MissingOutputType,
    UnknownFlag(String),
    NamedAsModule(String),
}

impl DefError {
    fn into_error(self, kind: DefKind, span: Span) -> ParseError {
        match self {
            Self::MissingName => ParseError::LabeledErrorWithHelp {
                error: "Missing definition name".into(),
                label: "expected a name here".into(),
                help: Self::signature_help(kind),
                span,
            },

            Self::MissingSignature => ParseError::LabeledErrorWithHelp {
                error: "Missing definition signature".into(),
                label: "expected a signature here".into(),
                help: Self::signature_help(kind),
                span,
            },

            Self::MissingBlock => ParseError::LabeledErrorWithHelp {
                error: "Missing definition block".into(),
                label: "expected a block here".into(),
                help: Self::signature_help(kind),
                span,
            },

            Self::InvalidName => ParseError::CommandDefNotValid(span),

            Self::MissingInputType => ParseError::LabeledErrorWithHelp {
                error: "Missing input type".into(),
                label: "expected a type here".into(),
                help:
                    "If you are not sure of what the command expects, consider using the `any` type"
                        .into(),
                span,
            },

            Self::MissingOutputType => ParseError::LabeledErrorWithHelp {
                error: "Missing output type".into(),
                label: "expected a type here".into(),
                help:
                    "If you are not sure of what the command returns, consider using the `any` type"
                        .into(),
                span,
            },

            Self::MissingArrow => ParseError::LabeledErrorWithHelp {
                error: "Missing input/output annotation arrow".into(),
                label: "expected a `->` here".into(),
                help: "An input/output annotation should look like this\n\tinput -> output".into(),
                span,
            },

            Self::UnclosedList => ParseError::Unclosed(String::from("]"), span),

            Self::Undeclared => ParseError::UnknownState(
                format!("internal error: `{}` declaration not found", kind.as_str()),
                span,
            ),

            Self::Duplicated => ParseError::DuplicateCommandDef(span),

            Self::NamedAsModule(module) => ParseError::NamedAsModule {
                item: kind.as_str().into(),
                module,
                alternative: String::from("main"),
                span,
            },

            Self::UnexpectedFlag => ParseError::LabeledErrorWithHelp {
                error: "Unexpected flag".into(),
                label: "this position cannot have a flag".into(),
                help: format!(
                    "Consider putting the flag after the `{}` keyword",
                    kind.as_str()
                ),
                span,
            },

            Self::ExtraTokens => ParseError::ExtraTokens(span),

            Self::UnknownFlag(flag) => {
                ParseError::UnknownFlag(kind.as_str().into(), flag, span, Self::flags_help(kind))
            }

            Self::ExternWithBlock => {
                ParseError::ExtraPositional("extern <def_name> <params>".into(), span)
            }
        }
    }

    const DEF_SIGNATURE_HELP: &str = "";
    const EXTERN_SIGNATURE_HELP: &str = "";

    fn signature_help(kind: DefKind) -> String {
        match kind {
            DefKind::Def => Self::DEF_SIGNATURE_HELP.into(),
            DefKind::Extern => Self::EXTERN_SIGNATURE_HELP.into(),
        }
    }

    const DEF_FLAGS_HELP: &str = "--help(-h)";
    const EXTERN_FLAGS_HELP: &str = "--help(-h), --env, --wrapped";

    fn flags_help(kind: DefKind) -> String {
        let available = match kind {
            DefKind::Def => Self::DEF_FLAGS_HELP,
            DefKind::Extern => Self::EXTERN_FLAGS_HELP,
        };

        format!("Available flags: {available}. Use `--help` for more information.")
    }
}
