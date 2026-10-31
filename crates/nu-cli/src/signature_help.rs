//! Signature help shared by the REPL status line and `nu --lsp`: which call
//! a position sits in, and how that call's signature reads.

use nu_protocol::{
    PositionalArg, Signature, SyntaxShape,
    ast::{Argument, Block, Call, Expr, Expression, Traverse},
    engine::StateWorkingSet,
};
use std::{fmt, ops::ControlFlow};

/// The innermost internal call around a position.
pub struct ActiveCall<'a> {
    /// The call, as parsed.
    pub call: &'a Call,
    /// The position is on the command's name rather than its arguments.
    pub in_head: bool,
}

impl ActiveCall<'_> {
    /// How many positional arguments end before `pos`, which is the index of
    /// the positional parameter an argument at `pos` fills.
    pub fn positionals_before(&self, pos: usize) -> usize {
        self.call
            .arguments
            .iter()
            .filter(|arg| !matches!(arg, Argument::Named(..)))
            .take_while(|arg| arg.span().end < pos)
            .count()
    }

    /// Whether `pos` is inside a flag or its value.
    pub fn in_flag(&self, pos: usize) -> bool {
        self.call
            .arguments
            .iter()
            .any(|arg| matches!(arg, Argument::Named(..)) && arg.span().contains(pos))
    }
}

/// The innermost internal call in `block` whose span contains `pos`.
///
/// A `pos` on a command's name finds that command when `on_name` is set.
/// Otherwise it finds the call that command is an argument of, the way
/// editors show `echo`'s signature with the cursor in `str` of
/// `echo (str length)`, and none at the top level.
pub fn find_active_call<'a>(
    block: &'a Block,
    working_set: &'a StateWorkingSet,
    pos: usize,
    on_name: bool,
) -> Option<ActiveCall<'a>> {
    block.find_map(working_set, &|expr| {
        find_active_call_in(expr, working_set, pos, on_name)
    })
}

/// [`find_active_call`] for one expression: `Continue` to look deeper,
/// `Break` with the call found, or with `None` when `expr` can't hold one.
fn find_active_call_in<'a>(
    expr: &'a Expression,
    working_set: &'a StateWorkingSet,
    pos: usize,
    on_name: bool,
) -> ControlFlow<Option<ActiveCall<'a>>> {
    if !expr.span.contains(pos) {
        return ControlFlow::Break(None);
    }
    let Expr::Call(call) = &expr.expr else {
        return ControlFlow::Continue(());
    };
    if call.head.contains(pos) {
        return ControlFlow::Break(on_name.then_some(ActiveCall {
            call,
            in_head: true,
        }));
    }
    let closure = |e| find_active_call_in(e, working_set, pos, on_name);
    let inner = call
        .arguments
        .iter()
        .find_map(|arg| arg.expr().and_then(|e| e.find_map(working_set, &closure)));
    ControlFlow::Break(Some(inner.unwrap_or(ActiveCall {
        call,
        in_head: false,
    })))
}

/// A signature as signature help shows it, `str substring {flags} <range>
/// ...(rest)`, kept in parts so one parameter can be picked out.
pub struct SignatureLabel {
    /// The name, `{flags}` when the command has any, then one part per
    /// positional parameter: required, optional, rest.
    pub parts: Vec<String>,
    has_flags: bool,
    positionals: usize,
    has_rest: bool,
}

impl SignatureLabel {
    /// The label of `signature`, as signature help shows it.
    pub fn new(signature: &Signature) -> Self {
        let expand_keyword = |arg: &PositionalArg, optional: bool| match &arg.shape {
            SyntaxShape::Keyword(kwd, _) => {
                format!("{} <{}>", String::from_utf8_lossy(kwd), arg.name)
            }
            _ if optional => arg.name.clone(),
            _ => format!("<{}>", arg.name),
        };
        let has_flags = !signature.named.is_empty();
        let mut parts = vec![signature.name.clone()];
        if has_flags {
            parts.push("{flags}".into());
        }
        parts.extend(
            signature
                .required_positional
                .iter()
                .map(|arg| expand_keyword(arg, false)),
        );
        parts.extend(
            signature
                .optional_positional
                .iter()
                .map(|arg| format!("({})", expand_keyword(arg, true))),
        );
        if let Some(arg) = &signature.rest_positional {
            parts.push(format!("...({})", arg.name));
        }
        Self {
            parts,
            has_flags,
            positionals: signature.required_positional.len() + signature.optional_positional.len(),
            has_rest: signature.rest_positional.is_some(),
        }
    }

    /// The part of the `index`th positional parameter, the rest parameter
    /// past the others, or `None` when the command takes no more.
    pub fn positional_part(&self, index: usize) -> Option<usize> {
        let first = 1 + usize::from(self.has_flags);
        if index < self.positionals {
            Some(first + index)
        } else {
            self.has_rest.then_some(first + self.positionals)
        }
    }

    /// The `{flags}` part, when the command has flags.
    pub fn flags_part(&self) -> Option<usize> {
        self.has_flags.then_some(1)
    }
}

impl fmt::Display for SignatureLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.parts.join(" "))
    }
}
