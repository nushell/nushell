//! Statements that read another file: `source` and `source-env`, `run`, and `plugin use` (the
//! classic parser's `parse_source`, `parse_run` and `parse_plugin_use`, in `parse_source.rs`).

use nu_protocol::{
    IntoSpanned, Span, Spanned, SyntaxShape, Type,
    ast::{Call, Expr, Expression, ListItem, Pipeline},
};

use super::{Lower, Lowered, Unlowered};
use crate::winnow::syntax::ast as w;
use crate::{
    parse_calls::CallKind,
    parse_def::has_flag_const,
    parse_expressions::parse_value as classic_value,
    parse_helpers::{extract_spread_list, extract_spread_record},
    parse_keywords::find_keyword_decl,
    parse_source::{finish_run, finish_source},
};

impl<'s> Lower<'_, '_, 's> {
    /// `source file` or `source-env file` (`parse_source`), the statement `e`: the call, its
    /// arguments read with the signature's shapes (`parse_internal_call`), then
    /// `finish_source`, which evaluates the file's name, parses the file (in the caller's
    /// scope for `source`, in one of its own for `source-env`) and gives its block to the call.
    ///
    /// As for `use`, once `finish_source` has run the statement is never handed back: the
    /// file's definitions are declared, and the classic parser would parse it again.
    pub(super) fn source(
        &mut self,
        e: &w::Expression<'s>,
        call: &w::Call<'s>,
    ) -> Lowered<Pipeline> {
        let name = call.head.name.as_ref();
        let decl_id = self
            .working_set
            .find_decl(name.as_bytes())
            .ok_or(Unlowered::Unsupported("source that is not a command"))?;
        let head = self.span(call.head.span);
        let (lowered, output, kind) = self.internal_call(head, &call.arguments, decl_id, None)?;
        let args_spans: Vec<Span> = call
            .arguments
            .iter()
            .map(|argument| self.span(argument.span()))
            .collect();
        Ok(finish_source(
            self.working_set,
            lowered,
            output,
            kind,
            self.span(e.span),
            &args_spans,
            name == "source-env",
        ))
    }

    /// `run script args` (`parse_run`, and `parse_run_expr` in a pipeline or after an
    /// environment shorthand): the call with `run`'s own arguments, up to the script's path,
    /// read with its signature (`parse_internal_call`), then the script's arguments
    /// (`add_script_arguments`), then `finish_run`, which parses the script and gives its block
    /// to the call. Once `finish_run` has parsed the script, which declares its definitions in
    /// the caller's scope, nothing may hand the statement back, so in a pipeline only the last
    /// element is lowered as a `run`.
    pub(super) fn run(&mut self, e: &w::Expression<'s>, call: &w::Call<'s>) -> Lowered<Expression> {
        let decl_id = find_keyword_decl(self.working_set, b"run")
            .or_else(|| self.working_set.find_decl(b"run"))
            .ok_or(Unlowered::Unsupported("run that is not a command"))?;
        // `run`'s own flags come before the script's path, the first item that is no flag or
        // spread, or the item after `--`; the script's arguments follow it.
        let arguments = &call.arguments;
        let own_end = match arguments.iter().position(|argument| {
            let span = self.span(argument.span());
            let word = self.text(argument.span()).as_bytes();
            let is_spread = extract_spread_record(word.into_spanned(span)).is_some()
                || extract_spread_list(word.into_spanned(span)).is_some();
            word == b"--" || !(word.starts_with(b"-") || is_spread)
        }) {
            Some(index) if self.text(arguments[index].span()) == "--" => {
                (index + 2).min(arguments.len())
            }
            Some(index) => index + 1,
            None => arguments.len(),
        };
        let (own, script) = arguments.split_at(own_end);
        let head = self.span(call.head.span);
        let (mut lowered, output, kind) = self.internal_call(head, own, decl_id, None)?;
        let do_full_reparse = if kind == CallKind::Help {
            Err(())
        } else {
            self.checked(|working_set| has_flag_const(working_set, &lowered, "full-reparse"))?
        };
        self.script_arguments(&mut lowered, script)?;
        let args_spans: Vec<Span> = arguments
            .iter()
            .map(|argument| self.span(argument.span()))
            .collect();
        Ok(finish_run(
            self.working_set,
            lowered,
            output,
            do_full_reparse,
            self.span(e.span),
            &args_spans,
        ))
    }

    /// The arguments after `run`'s script, added to its call as `add_script_arguments` adds
    /// them, each read as any value: a spread, its value in a one-item list; `--name=value`
    /// before any `--` (but `--help=...`), a named argument with its value in a one-item list;
    /// anything else, an unknown argument.
    fn script_arguments(&mut self, call: &mut Call, arguments: &[w::Argument<'s>]) -> Lowered<()> {
        let mut end_of_options = false;
        for argument in arguments {
            let span = self.span(argument.span());
            let word = self.text(argument.span());
            end_of_options |= word == "--";
            let spread_shape = if extract_spread_list(word.as_bytes().into_spanned(span)).is_some()
            {
                Some(SyntaxShape::Any)
            } else if extract_spread_record(word.as_bytes().into_spanned(span)).is_some() {
                Some(SyntaxShape::record())
            } else {
                None
            };
            let inline_flag = word
                .strip_prefix("--")
                .filter(|_| !end_of_options)
                .and_then(|flag| flag.split_once('='))
                .filter(|(name, _)| !name.is_empty() && *name != "help");
            match (argument, spread_shape, inline_flag) {
                (w::Argument::Spread { expr, .. }, Some(shape), _) => {
                    let value = self.value(expr, &shape, None)?;
                    let wrapped = self.one_item_list(value);
                    call.add_spread(wrapped);
                }
                (
                    w::Argument::Named(w::NamedArgument {
                        value: Some(value), ..
                    }),
                    None,
                    Some((name, _)),
                ) => {
                    let value = self.value(value, &SyntaxShape::Any, None)?;
                    let wrapped = self.one_item_list(value);
                    let name_span = Span::new(span.start, span.start + 2 + name.len());
                    let name = Spanned {
                        item: name.to_string(),
                        span: name_span,
                    };
                    call.add_named((name, None, Some(wrapped)));
                }
                (_, None, None) => {
                    let value = match argument {
                        w::Argument::Positional(expr) => {
                            self.value(expr, &SyntaxShape::Any, None)?
                        }
                        _ => self.checked(|ws| classic_value(ws, span, &SyntaxShape::Any, None))?,
                    };
                    call.add_unknown(value);
                }
                _ => {
                    return Err(Unlowered::Unsupported(
                        "script argument the tree reads otherwise",
                    ));
                }
            }
        }
        Ok(())
    }

    /// `value` in a list of one item, covering it, as `add_script_arguments` passes a spread's
    /// or a flag's value to `run`.
    fn one_item_list(&mut self, value: Expression) -> Expression {
        let span = value.span;
        let ty = Type::list(value.ty.clone());
        self.node(Expr::List(vec![ListItem::Item(value)]), span, ty)
    }
}
