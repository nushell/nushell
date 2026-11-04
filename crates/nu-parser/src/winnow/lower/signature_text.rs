//! Signatures read from their text, as nu-parser's `parse_full_signature`, `parse_signature`,
//! `parse_signature_helper` and `parse_input_output_types` read them: a signature nu reads from
//! a definition's items where the winnow parser read the definition as a plain call (`def f --
//! [x] {}`), and a definition's input and output types.

use nu_protocol::{
    Flag, ParseError, PositionalArg, Signature, Span, SyntaxShape, Type, VarId,
    ast::{Expr, Expression},
};

use super::{Lower, Lowered};
use crate::parse_def::reject_command_name;
use crate::parse_helpers::SPREAD_OPERATOR_STR;
use crate::parse_signatures::{Arg, assemble_signature, ensure_not_reserved_variable_name};
use crate::winnow::syntax::ast as w;
use crate::winnow::syntax::lex::{LexOptions, Token, TokenContents};
use crate::winnow::syntax::parse_helpers::is_variable;

/// What the next item of a signature is read as (nu's `ParseMode`).
#[derive(Clone, Copy)]
enum Mode {
    Arg,
    AfterCommaArg,
    Type,
    AfterType,
    DefaultValue,
}

/// A parameter's variable, declared once every parameter is read: its name and id.
type PendingVariable = (Vec<u8>, VarId);

/// What reading a signature's parameter adds to: the parameters read so far and the variables
/// to declare once all are read (none for an `extern`'s).
struct SignatureParams<'a> {
    external: bool,
    args: &'a mut Vec<Arg>,
    pending: &'a mut Vec<PendingVariable>,
}

impl Lower<'_, '_, '_> {
    /// The predeclarations of the statement at `span`, which the winnow parser read no
    /// definition in: each pipeline of one command lite-parsed from its text, as nu's
    /// `parse_block` predeclares them ([`Self::def_predecl_text`]).
    pub(in crate::winnow) fn predecl_statement_text(
        &mut self,
        span: Span,
        options: LexOptions,
    ) -> Lowered<()> {
        let (tokens, _) = self.lex_span(span, options);
        let (block, _) = self.lite_parse(&tokens);
        for pipeline in &block.pipelines {
            if let [command] = pipeline.as_slice() {
                self.def_predecl_text(command.command_parts())?;
            }
        }
        Ok(())
    }

    /// The predeclaration of a `def` or `extern` from the items `spans` of its statement, as
    /// `parse_def_predecl` reads them: the command's name, read as a string, after the flags,
    /// and its signature from the first item after it in `[...]` or `(...)`, read in a scope
    /// of its own with its errors dropped (the definition reports them). A `type` statement
    /// records its name and place, so that a use above it is reported as such.
    pub(super) fn def_predecl_text(&mut self, spans: &[Span]) -> Lowered<()> {
        if spans.len() < 3 {
            return Ok(());
        }
        let mut pos = if self.text_at(spans[0]) == "export" {
            2
        } else {
            1
        };
        let external = match self.text_at(spans[pos - 1]) {
            "def" => false,
            "extern" => true,
            "type" => {
                if let [name, equals, ..] = spans[pos..]
                    && self.text_at(equals) == "="
                {
                    let name = self.working_set.get_span_contents(name).to_vec();
                    self.working_set.add_type_predecl(name, Span::concat(spans));
                }
                return Ok(());
            }
            _ => return Ok(()),
        };
        while pos < spans.len() && self.text_at(spans[pos]).starts_with('-') {
            pos += 1;
        }
        let Some(&name_span) = spans.get(pos) else {
            return Ok(());
        };
        let name = w::Expression::new(w::Expr::Garbage, self.local(name_span));
        let Some(name) = self.string_leaf(&name)?.as_string() else {
            return Ok(());
        };
        if reject_command_name(self.working_set, &name, name_span) {
            return Ok(());
        }
        let Some(signature_pos) =
            (pos..spans.len()).find(|&pos| self.text_at(spans[pos]).starts_with(['[', '(']))
        else {
            return Ok(());
        };
        let wrapped = !external && spans.iter().any(|span| self.text_at(*span) == "--wrapped");
        let errors = self.working_set.parse_errors.len();
        self.working_set.enter_scope();
        let signature = self.full_signature_text(&spans[signature_pos..], external);
        self.working_set.parse_errors.truncate(errors);
        self.working_set.exit_scope();
        let Some(signature) = signature?.as_signature() else {
            return Ok(());
        };
        let params = self.text_at(spans[signature_pos]);
        self.declare_predecl(signature, name, name_span, params, wrapped, external);
        Ok(())
    }

    /// A signature read from the items `spans` (`parse_full_signature`): its parameters, and
    /// after a `:` its input and output types. A second item that is a `{...}` is dropped.
    pub(super) fn full_signature_text(
        &mut self,
        spans: &[Span],
        external: bool,
    ) -> Lowered<Expression> {
        match spans {
            [] => {
                self.working_set.error(ParseError::InternalError(
                    "failed to catch missing positional arguments".into(),
                    Span::concat(spans),
                ));
                Ok(Expression::garbage(self.working_set, Span::concat(spans)))
            }
            [only] => self.signature_text(*only, external),
            [first, second] if self.text_at(*second).starts_with('{') => {
                self.signature_text(*first, external)
            }
            [first, second, ..] => {
                let (mut signature, types_at) = if self.text_at(*first).ends_with(':') {
                    let span = Span::new(first.start, first.end.saturating_sub(1));
                    (self.signature_text(span, external)?, 1)
                } else if self.text_at(*second) == ":" {
                    (self.signature_text(*first, external)?, 2)
                } else {
                    self.working_set.error(ParseError::Expected(
                        "colon (:) before type signature",
                        Span::concat(&spans[1..]),
                    ));
                    (self.signature_text(*first, external)?, 1)
                };
                let types = self.input_output_types_text(&spans[types_at..])?;
                if let Expression {
                    expr: Expr::Signature(signature),
                    span,
                    ..
                } = &mut signature
                {
                    signature.input_output_types = types;
                    span.end = Span::concat(&spans[types_at..]).end;
                }
                Ok(signature)
            }
        }
    }

    /// A signature between `[...]` or `(...)` (`parse_signature`).
    pub(super) fn signature_text(&mut self, span: Span, external: bool) -> Lowered<Expression> {
        let text = self.text_at(span);
        let parenthesized = text.starts_with('(');
        if !parenthesized && !text.starts_with('[') {
            let at = Span::new(span.start, span.start + 1);
            self.working_set.error(ParseError::Expected("[ or (", at));
            return Ok(Expression::garbage(self.working_set, span));
        }
        let closed = match parenthesized {
            true => text.ends_with(')'),
            false => text.ends_with(']'),
        };
        let end = if closed {
            span.end - 1
        } else {
            let open = ParseError::opener_span(span, 1);
            let end = Span::new(span.end, span.end);
            self.working_set
                .error(ParseError::unclosed("] or )", open, end));
            span.end
        };
        let signature = self.signature_items(Span::new(span.start + 1, end), external)?;
        Ok(self.node(Expr::Signature(signature), span, Type::Any))
    }

    /// The parameters of the signature inside `span` (`parse_signature_helper`): each a
    /// positional (`x`, `x?`), a rest parameter (`...x`), a long flag with an optional short
    /// one (`--x(-y)`), a short flag (`-x`) or a short form for the last flag (`(-y)`), with a
    /// type after `:` (and a completer after its `@`) and a default value after `=`; comments
    /// describe the parameter before them. The variables are declared once every parameter is
    /// read, so that default values see the variables outside the signature.
    pub(super) fn signature_items(
        &mut self,
        span: Span,
        external: bool,
    ) -> Lowered<Box<Signature>> {
        let (tokens, error) = self.lex_span(span, LexOptions::SIGNATURE);
        if let Some(error) = error {
            self.working_set.error(error);
        }
        let mut args: Vec<Arg> = Vec::new();
        let mut mode = Mode::Arg;
        let mut pending: Vec<PendingVariable> = Vec::new();
        let expected = |this: &mut Self, what: &'static str, at: Span| {
            this.working_set.error(ParseError::Expected(what, at));
        };
        for (index, token) in tokens.iter().enumerate() {
            let last_token = index == tokens.len() - 1;
            let token_span = self.span(token.span);
            match token.contents {
                TokenContents::Item | TokenContents::AssignmentOperator(_) => {
                    let contents = self.text_at(token_span);
                    let point = Span::new(token_span.end, token_span.end);
                    match (contents, mode) {
                        // `:` separates a parameter from its type.
                        (":", Mode::Arg) if last_token => expected(self, "type", point),
                        (":", Mode::Arg) => mode = Mode::Type,
                        (":", Mode::AfterCommaArg | Mode::AfterType) => {
                            expected(self, "parameter or flag", token_span);
                        }
                        (":", Mode::Type | Mode::DefaultValue) => {
                            expected(self, "type", token_span);
                        }
                        // `=` separates a parameter from its default value.
                        ("=", Mode::Arg | Mode::AfterType) if last_token => {
                            expected(self, "default value", point);
                        }
                        ("=", Mode::Arg | Mode::AfterType) => mode = Mode::DefaultValue,
                        ("=", Mode::Type) => expected(self, "type", token_span),
                        ("=", Mode::AfterCommaArg) => {
                            expected(self, "parameter or flag", token_span);
                        }
                        ("=", Mode::DefaultValue) => expected(self, "default value", token_span),
                        // `,` separates parameters only.
                        (",", Mode::Arg | Mode::AfterType) => mode = Mode::AfterCommaArg,
                        (",", Mode::AfterCommaArg) => {
                            expected(self, "parameter or flag", token_span);
                        }
                        (",", Mode::Type) => expected(self, "type", token_span),
                        (",", Mode::DefaultValue) => expected(self, "default value", token_span),
                        (_, Mode::Arg | Mode::AfterCommaArg | Mode::AfterType) => {
                            let params = SignatureParams {
                                external,
                                args: &mut args,
                                pending: &mut pending,
                            };
                            self.signature_parameter(contents, token_span, &mut mode, params);
                        }
                        (_, Mode::Type) => {
                            self.signature_type(contents, token_span, external, &mut args)?;
                            mode = Mode::AfterType;
                        }
                        (_, Mode::DefaultValue) => {
                            if !external && let Some(last) = args.last_mut() {
                                let shape = last.default_value_shape();
                                let value = self.value_text(token.span, &shape)?;
                                last.set_default_value(self.working_set, value);
                            }
                            mode = Mode::Arg;
                        }
                    }
                }
                TokenContents::Comment => {
                    let text = self.text_at(Span::new(token_span.start + 1, token_span.end));
                    let description = match args.last_mut() {
                        Some(Arg::Flag { flag, .. }) => &mut flag.desc,
                        Some(Arg::Positional { arg, .. } | Arg::RestPositional(arg)) => {
                            &mut arg.desc
                        }
                        None => continue,
                    };
                    if !description.is_empty() {
                        description.push('\n');
                    }
                    description.push_str(text.trim());
                }
                _ => {}
            }
        }
        for (name, var_id) in pending {
            self.working_set.insert_variable_into_scope(name, var_id);
        }
        Ok(assemble_signature(self.working_set, args, span, external))
    }

    /// One parameter of a signature, `contents` at `span` (the parameter cases of
    /// `parse_signature_helper`). Every case but a short form for the last flag reads the next
    /// item as a parameter again.
    fn signature_parameter(
        &mut self,
        contents: &str,
        span: Span,
        mode: &mut Mode,
        params: SignatureParams<'_>,
    ) {
        let SignatureParams {
            external,
            args,
            pending,
        } = params;
        let mut add_variable = |this: &mut Self, name: Vec<u8>, ty: Type| {
            if external {
                return None;
            }
            ensure_not_reserved_variable_name(this.working_set, &name, span);
            let var_id = this.working_set.add_variable_without_scope(span, ty, false);
            pending.push((name, var_id));
            Some(var_id)
        };
        let expected = |this: &mut Self, what: &'static str| {
            this.working_set.error(ParseError::Expected(what, span));
        };
        let flag = |long: String, short: Option<char>, var_id| Arg::Flag {
            flag: Flag {
                arg: None,
                desc: String::new(),
                long,
                short,
                required: false,
                var_id,
                default_value: None,
                completion: None,
            },
            type_annotated: false,
        };
        let positional = |name: &str, var_id| PositionalArg {
            desc: String::new(),
            name: name.to_string(),
            shape: SyntaxShape::Any,
            var_id,
            default_value: None,
            completion: None,
        };
        // A short form for the last flag, `--b (-a)`, which leaves the mode as it is.
        if let Some(short) = contents.strip_prefix("(-") {
            if let Mode::AfterCommaArg = mode {
                expected(self, "parameter or flag");
            }
            let short = short.strip_suffix(')').unwrap_or_else(|| {
                expected(self, "short flag");
                short
            });
            let mut chars = short.chars();
            match (chars.next(), chars.next()) {
                (Some(short), None) => match args.last_mut() {
                    Some(Arg::Flag { flag, .. }) if flag.short.is_some() => {
                        expected(self, "one short flag");
                    }
                    Some(Arg::Flag { flag, .. }) => flag.short = Some(short),
                    _ => expected(self, "unknown flag"),
                },
                _ => expected(self, "short flag"),
            }
            return;
        }
        *mode = Mode::Arg;
        // A long flag, with a short one after it with no blank: `--output`, `--age(-a)`.
        if contents.starts_with("--") && contents.len() > 2 {
            let parts: Vec<&str> = contents.split('(').collect();
            let long = parts[0][2..].to_string();
            let variable = long.replace('-', "_");
            if !is_variable(&variable) {
                expected(self, "valid variable name for this long flag");
            }
            let var_id = add_variable(self, variable.into_bytes(), Type::Bool);
            match parts.as_slice() {
                [_] => args.push(flag(long, None, var_id)),
                [_, short] => {
                    let inner = short.strip_prefix('-').and_then(|s| s.strip_suffix(')'));
                    let short = inner.unwrap_or_else(|| {
                        expected(self, "short flag alternative for the long flag");
                        short
                    });
                    let mut chars = short.chars();
                    match (chars.next(), chars.next()) {
                        (Some(short), None) => args.push(flag(long, Some(short), var_id)),
                        _ => expected(self, "short flag"),
                    }
                }
                _ => expected(self, "only one short flag alternative"),
            }
        }
        // A short flag alone, `-e`: one character.
        else if let Some(short) = contents.strip_prefix('-')
            && let Some(first) = short.chars().next()
        {
            if short.chars().nth(1).is_some() {
                expected(self, "short flag");
            }
            let name = first.to_string();
            if !is_variable(&name) {
                expected(self, "valid variable name for this short flag");
            }
            let var_id = add_variable(self, name.into_bytes(), Type::Bool);
            args.push(flag(String::new(), Some(first), var_id));
        }
        // An optional positional, `x?`.
        else if let Some(name) = contents.strip_suffix('?') {
            if !is_variable(name) {
                expected(self, "valid variable name for this optional parameter");
            }
            let var_id = add_variable(self, name.as_bytes().to_vec(), Type::Any);
            args.push(Arg::Positional {
                arg: positional(name, var_id),
                required: false,
                type_annotated: false,
            });
        }
        // A rest parameter, `...x`.
        else if let Some(name) = contents.strip_prefix(SPREAD_OPERATOR_STR) {
            if !is_variable(name) {
                expected(self, "valid variable name for this rest parameter");
            }
            let var_id = add_variable(self, name.as_bytes().to_vec(), Type::Any);
            args.push(Arg::RestPositional(positional(name, var_id)));
        }
        // A required positional.
        else {
            if !is_variable(contents) {
                expected(self, "valid variable name for this parameter");
            }
            let var_id = add_variable(self, contents.as_bytes().to_vec(), Type::Any);
            args.push(Arg::Positional {
                arg: positional(contents, var_id),
                required: true,
                type_annotated: false,
            });
        }
    }

    /// The type after a parameter's `:`, `contents` at `span`, with a completer after an `@`
    /// (the type case of `parse_signature_helper`), given to the last parameter; none without
    /// a parameter before it.
    fn signature_type(
        &mut self,
        contents: &str,
        span: Span,
        external: bool,
        args: &mut [Arg],
    ) -> Lowered<()> {
        let Some(last) = args.last_mut() else {
            return Ok(());
        };
        // A completer needs a character after its `@`, or the `@` is read with the type.
        let split = contents.find('@').and_then(|at| {
            let (shape_span, completer_span) = span.split_at(at)?;
            let (_, completer_span) = completer_span.split_at(1)?;
            Some((&contents[..at], shape_span, completer_span))
        });
        let (shape, completion) = match split {
            Some((shape, shape_span, completer_span)) => {
                let shape = self.shape_name(shape, shape_span)?;
                (shape, self.completer_text(self.local(completer_span))?)
            }
            None => (self.shape_name(contents, span)?, None),
        };
        let ty = shape.to_type();
        let (var_id, var_type) = match last {
            Arg::Positional {
                arg,
                type_annotated,
                ..
            } => {
                *type_annotated = true;
                arg.completion = completion;
                arg.shape = shape;
                (arg.var_id, ty)
            }
            Arg::RestPositional(arg) => {
                arg.completion = completion;
                arg.shape = shape;
                (arg.var_id, Type::List(Box::new(ty)))
            }
            Arg::Flag {
                flag,
                type_annotated,
            } => {
                if shape == SyntaxShape::Boolean {
                    self.working_set.error(ParseError::LabeledError(
                        "Type annotations are not allowed for boolean switches.".into(),
                        "Remove the `: bool` type annotation.".into(),
                        span,
                    ));
                }
                *type_annotated = true;
                flag.completion = completion;
                flag.arg = Some(shape);
                (flag.var_id, ty)
            }
        };
        if !external && let Some(var_id) = var_id {
            self.working_set.set_variable_type(var_id, var_type);
        }
        Ok(())
    }

    /// A definition's input and output types from the items `spans`
    /// (`parse_input_output_types`): pairs of types with `->` between, in `[...]` or not.
    pub(super) fn input_output_types_text(&mut self, spans: &[Span]) -> Lowered<Vec<(Type, Type)>> {
        // None after `[]` and a lone `:` (nu reads them from the empty text at 0).
        if spans.is_empty() {
            return Ok(Vec::new());
        }
        let mut span = Span::concat(spans);
        let text = self.text_at(span);
        let inner = text.strip_prefix('[').unwrap_or(text);
        if inner.len() < text.len() {
            span.start += 1;
        }
        if inner.ends_with(']') {
            span.end -= 1;
        }
        let (tokens, error) = self.lex_span(span, LexOptions::IO_TYPES);
        if let Some(error) = error {
            self.working_set.error(error);
        }
        let tokens: Vec<Token> = tokens
            .into_iter()
            .filter(|token| token.contents != TokenContents::Comment)
            .collect();
        let end_of = |this: &Self, token: &Token| {
            let end = this.span(token.span).end;
            Span::new(end, end)
        };
        let mut types = Vec::new();
        let mut items = tokens.iter();
        while let Some(input) = items.next() {
            let input_span = self.span(input.span);
            let input_type = self.type_text(self.text_at(input_span), input_span)?;
            let Some(arrow) = items.next() else {
                let at = end_of(self, input);
                self.working_set
                    .error(ParseError::Expected("arrow (->)", at));
                break;
            };
            let arrow_span = self.span(arrow.span);
            if self.text_at(arrow_span) != "->" {
                self.working_set
                    .error(ParseError::Expected("arrow (->)", arrow_span));
            }
            let Some(output) = items.next() else {
                let at = end_of(self, arrow);
                self.working_set.error(ParseError::MissingType(at));
                break;
            };
            let output_span = self.span(output.span);
            let output_type = self.type_text(self.text_at(output_span), output_span)?;
            types.push((input_type, output_type));
        }
        Ok(types)
    }
}
