use std::ops::{Index, IndexMut};

use crate::{
    lex, lite_parse,
    parser_state::{Type, VarId},
    signature::Flag,
    DeclId, LiteBlock, ParseError, ParserWorkingSet, Signature, Span,
};

/// The syntactic shapes that values must match to be passed into a command. You can think of this as the type-checking that occurs when you call a function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyntaxShape {
    /// A specific match to a word or symbol
    Literal(Vec<u8>),

    /// Any syntactic form is allowed
    Any,

    /// Strings and string-like bare words are allowed
    String,

    /// A dotted path to navigate the table
    ColumnPath,

    /// A dotted path to navigate the table (including variable)
    FullColumnPath,

    /// Only a numeric (integer or decimal) value is allowed
    Number,

    /// A range is allowed (eg, `1..3`)
    Range,

    /// Only an integer value is allowed
    Int,

    /// A filepath is allowed
    FilePath,

    /// A glob pattern is allowed, eg `foo*`
    GlobPattern,

    /// A block is allowed, eg `{start this thing}`
    Block,

    /// A table is allowed, eg `[first second]`
    Table,

    /// A table is allowed, eg `[first second]`
    List(Box<SyntaxShape>),

    /// A filesize value is allowed, eg `10kb`
    Filesize,

    /// A duration value is allowed, eg `19day`
    Duration,

    /// An operator
    Operator,

    /// A math expression which expands shorthand forms on the lefthand side, eg `foo > 1`
    /// The shorthand allows us to more easily reach columns inside of the row being passed in
    RowCondition,

    /// A general math expression, eg `1 + 2`
    MathExpression,

    /// A variable name
    Variable,

    /// A general expression, eg `1 + 2` or `foo --bar`
    Expression,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operator {
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessThanOrEqual,
    GreaterThanOrEqual,
    Contains,
    NotContains,
    Plus,
    Minus,
    Multiply,
    Divide,
    In,
    NotIn,
    Modulo,
    And,
    Or,
    Pow,
}

#[derive(Debug, Clone)]
pub struct Call {
    /// identifier of the declaration to call
    pub decl_id: DeclId,
    pub positional: Vec<Expression>,
    pub named: Vec<(String, Option<Expression>)>,
}

impl Default for Call {
    fn default() -> Self {
        Self::new()
    }
}

impl Call {
    pub fn new() -> Call {
        Self {
            decl_id: 0,
            positional: vec![],
            named: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64),
    Var(VarId),
    Call(Box<Call>),
    ExternalCall(Vec<u8>, Vec<Vec<u8>>),
    Operator(Operator),
    BinaryOp(Box<Expression>, Box<Expression>, Box<Expression>), //lhs, op, rhs
    Subexpression(Box<Block>),
    Block(Box<Block>),
    List(Vec<Expression>),
    Table(Vec<Expression>, Vec<Vec<Expression>>),
    Literal(Vec<u8>),
    String(String), // FIXME: improve this in the future?
    Garbage,
}

#[derive(Debug, Clone)]
pub struct Expression {
    expr: Expr,
    span: Span,
}
impl Expression {
    pub fn garbage(span: Span) -> Expression {
        Expression {
            expr: Expr::Garbage,
            span,
            //ty: Type::Unknown,
        }
    }
    pub fn precedence(&self) -> usize {
        match &self.expr {
            Expr::Operator(operator) => {
                // Higher precedence binds tighter

                match operator {
                    Operator::Pow => 100,
                    Operator::Multiply | Operator::Divide | Operator::Modulo => 95,
                    Operator::Plus | Operator::Minus => 90,
                    Operator::NotContains
                    | Operator::Contains
                    | Operator::LessThan
                    | Operator::LessThanOrEqual
                    | Operator::GreaterThan
                    | Operator::GreaterThanOrEqual
                    | Operator::Equal
                    | Operator::NotEqual
                    | Operator::In
                    | Operator::NotIn => 80,
                    Operator::And => 50,
                    Operator::Or => 40, // TODO: should we have And and Or be different precedence?
                }
            }
            _ => 0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Import {}

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Statement>,
}

impl Block {
    pub fn len(&self) -> usize {
        self.stmts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.stmts.is_empty()
    }
}

impl Index<usize> for Block {
    type Output = Statement;

    fn index(&self, index: usize) -> &Self::Output {
        &self.stmts[index]
    }
}

impl IndexMut<usize> for Block {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.stmts[index]
    }
}

impl Default for Block {
    fn default() -> Self {
        Self::new()
    }
}

impl Block {
    pub fn new() -> Self {
        Self { stmts: vec![] }
    }
}

#[derive(Debug, Clone)]
pub struct VarDecl {
    var_id: VarId,
    expression: Expression,
}

#[derive(Debug, Clone)]
pub enum Statement {
    Pipeline(Pipeline),
    VarDecl(VarDecl),
    Import(Import),
    Expression(Expression),
    None,
}

#[derive(Debug, Clone)]
pub struct Pipeline {}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl Pipeline {
    pub fn new() -> Self {
        Self {}
    }
}

fn garbage(span: Span) -> Expression {
    Expression::garbage(span)
}

fn is_identifier_byte(b: u8) -> bool {
    b != b'.' && b != b'[' && b != b'(' && b != b'{'
}

fn is_identifier(bytes: &[u8]) -> bool {
    bytes.iter().all(|x| is_identifier_byte(*x))
}

fn is_variable(bytes: &[u8]) -> bool {
    if bytes.len() > 1 && bytes[0] == b'$' {
        is_identifier(&bytes[1..])
    } else {
        is_identifier(bytes)
    }
}

fn check_call(command: Span, sig: &Signature, call: &Call) -> Option<ParseError> {
    if call.positional.len() < sig.required_positional.len() {
        let missing = &sig.required_positional[call.positional.len()];
        Some(ParseError::MissingPositional(missing.name.clone(), command))
    } else {
        for req_flag in sig.named.iter().filter(|x| x.required) {
            if call.named.iter().all(|(n, _)| n != &req_flag.long) {
                return Some(ParseError::MissingRequiredFlag(
                    req_flag.long.clone(),
                    command,
                ));
            }
        }
        None
    }
}

fn span(spans: &[Span]) -> Span {
    let length = spans.len();

    if length == 0 {
        Span::unknown()
    } else if length == 1 {
        spans[0]
    } else {
        Span {
            start: spans[0].start,
            end: spans[length - 1].end,
        }
    }
}

impl ParserWorkingSet {
    pub fn parse_external_call(&mut self, spans: &[Span]) -> (Expression, Option<ParseError>) {
        // TODO: add external parsing
        let mut args = vec![];
        let name = self.get_span_contents(spans[0]).to_vec();
        for span in &spans[1..] {
            args.push(self.get_span_contents(*span).to_vec());
        }
        (
            Expression {
                expr: Expr::ExternalCall(name, args),
                span: span(spans),
            },
            None,
        )
    }

    fn parse_long_flag(
        &mut self,
        spans: &[Span],
        spans_idx: &mut usize,
        sig: &Signature,
    ) -> (Option<String>, Option<Expression>, Option<ParseError>) {
        let arg_span = spans[*spans_idx];
        let arg_contents = self.get_span_contents(arg_span);

        if arg_contents.starts_with(&[b'-', b'-']) {
            // FIXME: only use the first you find
            let split: Vec<_> = arg_contents.split(|x| *x == b'=').collect();
            let long_name = String::from_utf8(split[0].into());
            if let Ok(long_name) = long_name {
                if let Some(flag) = sig.get_long_flag(&long_name) {
                    if let Some(arg_shape) = &flag.arg {
                        if split.len() > 1 {
                            // and we also have the argument
                            let mut span = arg_span;
                            span.start += long_name.len() + 1; //offset by long flag and '='
                            let (arg, err) = self.parse_value(span, arg_shape.clone());

                            (Some(long_name), Some(arg), err)
                        } else if let Some(arg) = spans.get(*spans_idx + 1) {
                            let (arg, err) = self.parse_value(*arg, arg_shape.clone());

                            *spans_idx += 1;
                            (Some(long_name), Some(arg), err)
                        } else {
                            (
                                Some(long_name),
                                None,
                                Some(ParseError::MissingFlagParam(arg_span)),
                            )
                        }
                    } else {
                        // A flag with no argument
                        (Some(long_name), None, None)
                    }
                } else {
                    (
                        Some(long_name),
                        None,
                        Some(ParseError::UnknownFlag(arg_span)),
                    )
                }
            } else {
                (Some("--".into()), None, Some(ParseError::NonUtf8(arg_span)))
            }
        } else {
            (None, None, None)
        }
    }

    fn parse_short_flags(
        &mut self,
        spans: &[Span],
        spans_idx: &mut usize,
        positional_idx: usize,
        sig: &Signature,
    ) -> (Option<Vec<Flag>>, Option<ParseError>) {
        let mut error = None;
        let arg_span = spans[*spans_idx];

        let arg_contents = self.get_span_contents(arg_span);

        if arg_contents.starts_with(&[b'-']) && arg_contents.len() > 1 {
            let short_flags = &arg_contents[1..];
            let mut found_short_flags = vec![];
            let mut unmatched_short_flags = vec![];
            for short_flag in short_flags.iter().enumerate() {
                let short_flag_char = char::from(*short_flag.1);
                let orig = arg_span;
                let short_flag_span = Span {
                    start: orig.start + 1 + short_flag.0,
                    end: orig.start + 1 + short_flag.0 + 1,
                };
                if let Some(flag) = sig.get_short_flag(short_flag_char) {
                    // If we require an arg and are in a batch of short flags, error
                    if !found_short_flags.is_empty() && flag.arg.is_some() {
                        error =
                            error.or(Some(ParseError::ShortFlagBatchCantTakeArg(short_flag_span)))
                    }
                    found_short_flags.push(flag);
                } else {
                    unmatched_short_flags.push(short_flag_span);
                }
            }

            if found_short_flags.is_empty() {
                // check to see if we have a negative number
                if let Some(positional) = sig.get_positional(positional_idx) {
                    if positional.shape == SyntaxShape::Int
                        || positional.shape == SyntaxShape::Number
                    {
                        if String::from_utf8_lossy(&arg_contents)
                            .parse::<f64>()
                            .is_ok()
                        {
                            return (None, None);
                        } else if let Some(first) = unmatched_short_flags.first() {
                            error = error.or(Some(ParseError::UnknownFlag(*first)));
                        }
                    } else if let Some(first) = unmatched_short_flags.first() {
                        error = error.or(Some(ParseError::UnknownFlag(*first)));
                    }
                } else if let Some(first) = unmatched_short_flags.first() {
                    error = error.or(Some(ParseError::UnknownFlag(*first)));
                }
            } else if !unmatched_short_flags.is_empty() {
                if let Some(first) = unmatched_short_flags.first() {
                    error = error.or(Some(ParseError::UnknownFlag(*first)));
                }
            }

            (Some(found_short_flags), error)
        } else {
            (None, None)
        }
    }

    fn parse_multispan_value(
        &mut self,
        spans: &[Span],
        spans_idx: &mut usize,
        shape: SyntaxShape,
    ) -> (Expression, Option<ParseError>) {
        let mut error = None;
        let arg_span = spans[*spans_idx];

        match shape {
            SyntaxShape::RowCondition => {
                let (arg, err) = self.parse_row_condition(spans);
                error = error.or(err);
                *spans_idx = spans.len();

                (arg, error)
            }
            SyntaxShape::Expression => {
                let (arg, err) = self.parse_expression(spans);
                error = error.or(err);
                *spans_idx = spans.len();

                (arg, error)
            }
            SyntaxShape::Literal(literal) => {
                let arg_contents = self.get_span_contents(arg_span);
                if arg_contents != literal {
                    // When keywords mismatch, this is a strong indicator of something going wrong.
                    // We won't often override the current error, but as this is a strong indicator
                    // go ahead and override the current error and tell the user about the missing
                    // keyword/literal.
                    error = Some(ParseError::Mismatch(
                        String::from_utf8_lossy(&literal).into(),
                        arg_span,
                    ))
                }
                (
                    Expression {
                        expr: Expr::Literal(literal),
                        span: arg_span,
                    },
                    error,
                )
            }
            _ => {
                // All other cases are single-span values
                let (arg, err) = self.parse_value(arg_span, shape);
                error = error.or(err);

                (arg, error)
            }
        }
    }

    pub fn parse_internal_call(
        &mut self,
        spans: &[Span],
        decl_id: usize,
    ) -> (Box<Call>, Span, Option<ParseError>) {
        let mut error = None;

        let mut call = Call::new();
        call.decl_id = decl_id;

        let sig = self
            .get_decl(decl_id)
            .expect("internal error: bad DeclId")
            .clone();

        // The index into the positional parameter in the definition
        let mut positional_idx = 0;

        // The index into the spans of argument data given to parse
        // Starting at the first argument
        let mut spans_idx = 1;

        while spans_idx < spans.len() {
            let arg_span = spans[spans_idx];

            let (long_name, arg, err) = self.parse_long_flag(spans, &mut spans_idx, &sig);
            if let Some(long_name) = long_name {
                // We found a long flag, like --bar
                error = error.or(err);
                call.named.push((long_name, arg));
                spans_idx += 1;
                continue;
            }

            let (short_flags, err) =
                self.parse_short_flags(spans, &mut spans_idx, positional_idx, &sig);

            if let Some(short_flags) = short_flags {
                error = error.or(err);
                for flag in short_flags {
                    if let Some(arg_shape) = flag.arg {
                        if let Some(arg) = spans.get(spans_idx + 1) {
                            let (arg, err) = self.parse_value(*arg, arg_shape.clone());
                            error = error.or(err);

                            call.named.push((flag.long.clone(), Some(arg)));
                            spans_idx += 1;
                        } else {
                            error = error.or(Some(ParseError::MissingFlagParam(arg_span)))
                        }
                    } else {
                        call.named.push((flag.long.clone(), None));
                    }
                }
                spans_idx += 1;
                continue;
            }

            if let Some(positional) = sig.get_positional(positional_idx) {
                //Make sure we leave enough spans for the remaining positionals
                let remainder = sig.num_positionals() - positional_idx;

                let (arg, err) = self.parse_multispan_value(
                    &spans[..(spans.len() - remainder + 1)],
                    &mut spans_idx,
                    positional.shape,
                );
                error = error.or(err);
                call.positional.push(arg);
                positional_idx += 1;
            } else {
                error = error.or(Some(ParseError::ExtraPositional(arg_span)))
            }

            error = error.or(err);
            spans_idx += 1;
        }

        let err = check_call(spans[0], &sig, &call);
        error = error.or(err);

        // FIXME: type unknown
        (Box::new(call), span(spans), error)
    }

    pub fn parse_call(&mut self, spans: &[Span]) -> (Expression, Option<ParseError>) {
        // assume spans.len() > 0?
        let name = self.get_span_contents(spans[0]);

        if let Some(decl_id) = self.find_decl(name) {
            let (call, span, err) = self.parse_internal_call(spans, decl_id);
            (
                Expression {
                    expr: Expr::Call(call),
                    span,
                },
                err,
            )
        } else {
            self.parse_external_call(spans)
        }
    }

    pub fn parse_int(&mut self, token: &str, span: Span) -> (Expression, Option<ParseError>) {
        if let Some(token) = token.strip_prefix("0x") {
            if let Ok(v) = i64::from_str_radix(token, 16) {
                (
                    Expression {
                        expr: Expr::Int(v),
                        span,
                    },
                    None,
                )
            } else {
                (
                    garbage(span),
                    Some(ParseError::Mismatch("int".into(), span)),
                )
            }
        } else if let Some(token) = token.strip_prefix("0b") {
            if let Ok(v) = i64::from_str_radix(token, 2) {
                (
                    Expression {
                        expr: Expr::Int(v),
                        span,
                    },
                    None,
                )
            } else {
                (
                    garbage(span),
                    Some(ParseError::Mismatch("int".into(), span)),
                )
            }
        } else if let Some(token) = token.strip_prefix("0o") {
            if let Ok(v) = i64::from_str_radix(token, 8) {
                (
                    Expression {
                        expr: Expr::Int(v),
                        span,
                    },
                    None,
                )
            } else {
                (
                    garbage(span),
                    Some(ParseError::Mismatch("int".into(), span)),
                )
            }
        } else if let Ok(x) = token.parse::<i64>() {
            (
                Expression {
                    expr: Expr::Int(x),
                    span,
                },
                None,
            )
        } else {
            (
                garbage(span),
                Some(ParseError::Mismatch("int".into(), span)),
            )
        }
    }

    pub fn parse_number(&mut self, token: &str, span: Span) -> (Expression, Option<ParseError>) {
        if let (x, None) = self.parse_int(token, span) {
            (x, None)
        } else {
            (
                garbage(span),
                Some(ParseError::Mismatch("number".into(), span)),
            )
        }
    }

    pub(crate) fn parse_dollar_expr(&mut self, span: Span) -> (Expression, Option<ParseError>) {
        let bytes = self.get_span_contents(span);

        if let Some(var_id) = self.find_variable(bytes) {
            (
                Expression {
                    expr: Expr::Var(var_id),
                    span,
                },
                None,
            )
        } else {
            (garbage(span), Some(ParseError::VariableNotFound(span)))
        }
    }

    pub fn parse_variable_expr(&mut self, span: Span) -> (Expression, Option<ParseError>) {
        let (id, err) = self.parse_variable(span);

        if err.is_none() {
            if let Some(id) = id {
                (
                    Expression {
                        expr: Expr::Var(id),
                        span,
                    },
                    None,
                )
            } else {
                let name = self.get_span_contents(span).to_vec();
                // this seems okay to set it to unknown here, but we should double-check
                let id = self.add_variable(name, Type::Unknown);

                (
                    Expression {
                        expr: Expr::Var(id),
                        span,
                    },
                    None,
                )
            }
        } else {
            (garbage(span), err)
        }
    }

    pub fn parse_full_column_path(&mut self, span: Span) -> (Expression, Option<ParseError>) {
        // FIXME: assume for now a paren expr, but needs more
        let bytes = self.get_span_contents(span);
        let mut error = None;

        let mut start = span.start;
        let mut end = span.end;

        if bytes.starts_with(b"(") {
            start += 1;
        }
        if bytes.ends_with(b")") {
            end -= 1;
        } else {
            error = error.or_else(|| {
                Some(ParseError::Unclosed(
                    ")".into(),
                    Span {
                        start: end,
                        end: end + 1,
                    },
                ))
            });
        }

        let span = Span { start, end };

        let source = self.get_span_contents(span);

        let (output, err) = lex(&source, start, crate::LexMode::Normal);
        error = error.or(err);

        let (output, err) = lite_parse(&output);
        error = error.or(err);

        let (output, err) = self.parse_block(&output);
        error = error.or(err);

        (
            Expression {
                expr: Expr::Subexpression(Box::new(output)),
                span,
            },
            error,
        )
    }

    pub fn parse_string(&mut self, span: Span) -> (Expression, Option<ParseError>) {
        let bytes = self.get_span_contents(span);

        if let Ok(token) = String::from_utf8(bytes.into()) {
            (
                Expression {
                    expr: Expr::String(token),
                    span,
                },
                None,
            )
        } else {
            (
                garbage(span),
                Some(ParseError::Mismatch("string".into(), span)),
            )
        }
    }

    pub fn parse_row_condition(&mut self, spans: &[Span]) -> (Expression, Option<ParseError>) {
        self.parse_math_expression(spans)
    }

    pub fn parse_list_expression(
        &mut self,
        span: Span,
        element_shape: &SyntaxShape,
    ) -> (Expression, Option<ParseError>) {
        let bytes = self.get_span_contents(span);

        let mut error = None;

        let mut start = span.start;
        let mut end = span.end;

        if bytes.starts_with(b"[") {
            start += 1;
        }
        if bytes.ends_with(b"]") {
            end -= 1;
        } else {
            error = error.or_else(|| {
                Some(ParseError::Unclosed(
                    "]".into(),
                    Span {
                        start: end,
                        end: end + 1,
                    },
                ))
            });
        }

        let span = Span { start, end };
        let source = &self.file_contents[..span.end];

        let (output, err) = lex(&source, span.start, crate::LexMode::CommaAndNewlineIsSpace);
        error = error.or(err);

        let (output, err) = lite_parse(&output);
        error = error.or(err);

        println!("{:?}", output);

        let mut args = vec![];
        for arg in &output.block[0].commands {
            for part in &arg.parts {
                let (arg, err) = self.parse_value(*part, element_shape.clone());
                error = error.or(err);

                args.push(arg);
            }
        }

        (
            Expression {
                expr: Expr::List(args),
                span,
            },
            error,
        )
    }

    pub fn parse_table_expression(&mut self, span: Span) -> (Expression, Option<ParseError>) {
        let bytes = self.get_span_contents(span);
        let mut error = None;

        let mut start = span.start;
        let mut end = span.end;

        if bytes.starts_with(b"[") {
            start += 1;
        }
        if bytes.ends_with(b"]") {
            end -= 1;
        } else {
            error = error.or_else(|| {
                Some(ParseError::Unclosed(
                    "]".into(),
                    Span {
                        start: end,
                        end: end + 1,
                    },
                ))
            });
        }

        let span = Span { start, end };

        let source = &self.file_contents[..end];

        let (output, err) = lex(&source, start, crate::LexMode::CommaAndNewlineIsSpace);
        error = error.or(err);

        let (output, err) = lite_parse(&output);
        error = error.or(err);

        match output.block.len() {
            0 => (
                Expression {
                    expr: Expr::List(vec![]),
                    span,
                },
                None,
            ),
            1 => {
                // List
                self.parse_list_expression(span, &SyntaxShape::Any)
            }
            _ => {
                let mut table_headers = vec![];

                let (headers, err) =
                    self.parse_value(output.block[0].commands[0].parts[0], SyntaxShape::Table);
                error = error.or(err);

                if let Expression {
                    expr: Expr::List(headers),
                    ..
                } = headers
                {
                    table_headers = headers;
                }

                let mut rows = vec![];
                for part in &output.block[1].commands[0].parts {
                    let (values, err) = self.parse_value(*part, SyntaxShape::Table);
                    error = error.or(err);
                    if let Expression {
                        expr: Expr::List(values),
                        ..
                    } = values
                    {
                        rows.push(values);
                    }
                }

                (
                    Expression {
                        expr: Expr::Table(table_headers, rows),
                        span,
                    },
                    error,
                )
            }
        }
    }

    pub fn parse_block_expression(&mut self, span: Span) -> (Expression, Option<ParseError>) {
        let bytes = self.get_span_contents(span);
        let mut error = None;

        let mut start = span.start;
        let mut end = span.end;

        if bytes.starts_with(b"{") {
            start += 1;
        } else {
            return (
                garbage(span),
                Some(ParseError::Mismatch("block".into(), span)),
            );
        }
        if bytes.ends_with(b"}") {
            end -= 1;
        } else {
            error = error.or_else(|| {
                Some(ParseError::Unclosed(
                    "}".into(),
                    Span {
                        start: end,
                        end: end + 1,
                    },
                ))
            });
        }

        let span = Span { start, end };

        let source = &self.file_contents[..end];

        let (output, err) = lex(&source, start, crate::LexMode::Normal);
        error = error.or(err);

        let (output, err) = lite_parse(&output);
        error = error.or(err);

        let (output, err) = self.parse_block(&output);
        error = error.or(err);

        println!("{:?} {:?}", output, error);

        (
            Expression {
                expr: Expr::Block(Box::new(output)),
                span,
            },
            error,
        )
    }

    pub fn parse_value(
        &mut self,
        span: Span,
        shape: SyntaxShape,
    ) -> (Expression, Option<ParseError>) {
        let bytes = self.get_span_contents(span);

        // First, check the special-cases. These will likely represent specific values as expressions
        // and may fit a variety of shapes.
        //
        // We check variable first because immediately following we check for variables with column paths
        // which might result in a value that fits other shapes (and require the variable to already be
        // declared)
        if shape == SyntaxShape::Variable {
            return self.parse_variable_expr(span);
        } else if bytes.starts_with(b"$") {
            return self.parse_dollar_expr(span);
        } else if bytes.starts_with(b"(") {
            return self.parse_full_column_path(span);
        }

        match shape {
            SyntaxShape::Number => {
                if let Ok(token) = String::from_utf8(bytes.into()) {
                    self.parse_number(&token, span)
                } else {
                    (
                        garbage(span),
                        Some(ParseError::Mismatch("number".into(), span)),
                    )
                }
            }
            SyntaxShape::Int => {
                if let Ok(token) = String::from_utf8(bytes.into()) {
                    self.parse_int(&token, span)
                } else {
                    (
                        garbage(span),
                        Some(ParseError::Mismatch("int".into(), span)),
                    )
                }
            }
            SyntaxShape::Literal(literal) => {
                if bytes == literal {
                    (
                        Expression {
                            expr: Expr::Literal(literal),
                            span,
                        },
                        None,
                    )
                } else {
                    (
                        garbage(span),
                        Some(ParseError::Mismatch(
                            format!("keyword '{}'", String::from_utf8_lossy(&literal)),
                            span,
                        )),
                    )
                }
            }
            SyntaxShape::String | SyntaxShape::GlobPattern | SyntaxShape::FilePath => {
                self.parse_string(span)
            }
            SyntaxShape::Block => {
                if bytes.starts_with(b"{") {
                    self.parse_block_expression(span)
                } else {
                    (
                        Expression::garbage(span),
                        Some(ParseError::Mismatch("table".into(), span)),
                    )
                }
            }
            SyntaxShape::List(elem) => self.parse_list_expression(span, &elem),
            SyntaxShape::Table => {
                if bytes.starts_with(b"[") {
                    self.parse_table_expression(span)
                } else {
                    (
                        Expression::garbage(span),
                        Some(ParseError::Mismatch("table".into(), span)),
                    )
                }
            }
            SyntaxShape::Any => {
                let shapes = vec![
                    SyntaxShape::Int,
                    SyntaxShape::Number,
                    SyntaxShape::Range,
                    SyntaxShape::Filesize,
                    SyntaxShape::Duration,
                    SyntaxShape::Block,
                    SyntaxShape::Table,
                    SyntaxShape::String,
                ];
                for shape in shapes.iter() {
                    if let (s, None) = self.parse_value(span, shape.clone()) {
                        return (s, None);
                    }
                }
                (
                    garbage(span),
                    Some(ParseError::Mismatch("any shape".into(), span)),
                )
            }
            _ => (
                garbage(span),
                Some(ParseError::Mismatch("incomplete parser".into(), span)),
            ),
        }
    }

    pub fn parse_operator(&mut self, span: Span) -> (Expression, Option<ParseError>) {
        let contents = self.get_span_contents(span);

        let operator = match contents {
            b"==" => Operator::Equal,
            b"!=" => Operator::NotEqual,
            b"<" => Operator::LessThan,
            b"<=" => Operator::LessThanOrEqual,
            b">" => Operator::GreaterThan,
            b">=" => Operator::GreaterThanOrEqual,
            b"=~" => Operator::Contains,
            b"!~" => Operator::NotContains,
            b"+" => Operator::Plus,
            b"-" => Operator::Minus,
            b"*" => Operator::Multiply,
            b"/" => Operator::Divide,
            b"in" => Operator::In,
            b"not-in" => Operator::NotIn,
            b"mod" => Operator::Modulo,
            b"&&" => Operator::And,
            b"||" => Operator::Or,
            b"**" => Operator::Pow,
            _ => {
                return (
                    garbage(span),
                    Some(ParseError::Mismatch("operator".into(), span)),
                );
            }
        };

        (
            Expression {
                expr: Expr::Operator(operator),
                span,
            },
            None,
        )
    }

    pub fn parse_math_expression(&mut self, spans: &[Span]) -> (Expression, Option<ParseError>) {
        // As the expr_stack grows, we increase the required precedence to grow larger
        // If, at any time, the operator we're looking at is the same or lower precedence
        // of what is in the expression stack, we collapse the expression stack.
        //
        // This leads to an expression stack that grows under increasing precedence and collapses
        // under decreasing/sustained precedence
        //
        // The end result is a stack that we can fold into binary operations as right associations
        // safely.

        let mut expr_stack: Vec<Expression> = vec![];

        let mut idx = 0;
        let mut last_prec = 1000000;

        let mut error = None;
        let (lhs, err) = self.parse_value(spans[0], SyntaxShape::Any);
        error = error.or(err);
        idx += 1;

        expr_stack.push(lhs);

        while idx < spans.len() {
            let (op, err) = self.parse_operator(spans[idx]);
            error = error.or(err);

            let op_prec = op.precedence();

            idx += 1;

            if idx == spans.len() {
                // Handle broken math expr `1 +` etc
                error = error.or(Some(ParseError::IncompleteMathExpression(spans[idx - 1])));
                break;
            }

            let (rhs, err) = self.parse_value(spans[idx], SyntaxShape::Any);
            error = error.or(err);

            if op_prec <= last_prec {
                while expr_stack.len() > 1 {
                    // Collapse the right associated operations first
                    // so that we can get back to a stack with a lower precedence
                    let rhs = expr_stack
                        .pop()
                        .expect("internal error: expression stack empty");
                    let op = expr_stack
                        .pop()
                        .expect("internal error: expression stack empty");
                    let lhs = expr_stack
                        .pop()
                        .expect("internal error: expression stack empty");

                    let op_span = span(&[lhs.span, rhs.span]);
                    expr_stack.push(Expression {
                        expr: Expr::BinaryOp(Box::new(lhs), Box::new(op), Box::new(rhs)),
                        span: op_span,
                    });
                }
            }
            expr_stack.push(op);
            expr_stack.push(rhs);

            last_prec = op_prec;

            idx += 1;
        }

        while expr_stack.len() != 1 {
            let rhs = expr_stack
                .pop()
                .expect("internal error: expression stack empty");
            let op = expr_stack
                .pop()
                .expect("internal error: expression stack empty");
            let lhs = expr_stack
                .pop()
                .expect("internal error: expression stack empty");

            let binary_op_span = span(&[lhs.span, rhs.span]);
            expr_stack.push(Expression {
                expr: Expr::BinaryOp(Box::new(lhs), Box::new(op), Box::new(rhs)),
                span: binary_op_span,
            });
        }

        let output = expr_stack
            .pop()
            .expect("internal error: expression stack empty");

        (output, error)
    }

    pub fn parse_expression(&mut self, spans: &[Span]) -> (Expression, Option<ParseError>) {
        let bytes = self.get_span_contents(spans[0]);

        match bytes[0] {
            b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9' | b'(' | b'{'
            | b'[' | b'$' => self.parse_math_expression(spans),
            _ => self.parse_call(spans),
        }
    }

    pub fn parse_variable(&mut self, span: Span) -> (Option<VarId>, Option<ParseError>) {
        let bytes = self.get_span_contents(span);

        if is_variable(bytes) {
            if let Some(var_id) = self.find_variable(bytes) {
                (Some(var_id), None)
            } else {
                (None, None)
            }
        } else {
            (None, Some(ParseError::Mismatch("variable".into(), span)))
        }
    }

    pub fn parse_keyword(&self, span: Span, keyword: &[u8]) -> Option<ParseError> {
        if self.get_span_contents(span) == keyword {
            None
        } else {
            Some(ParseError::Mismatch(
                String::from_utf8_lossy(keyword).to_string(),
                span,
            ))
        }
    }

    pub fn parse_let(&mut self, spans: &[Span]) -> (Statement, Option<ParseError>) {
        let name = self.get_span_contents(spans[0]);

        if name == b"let" {
            if let Some(decl_id) = self.find_decl(b"let") {
                let (mut call, call_span, err) = self.parse_internal_call(spans, decl_id);

                if err.is_some() {
                    return (
                        Statement::Expression(Expression {
                            expr: Expr::Call(call),
                            span: call_span,
                        }),
                        err,
                    );
                } else if let Expression {
                    expr: Expr::Var(var_id),
                    ..
                } = call.positional[0]
                {
                    let expression = call.positional.swap_remove(2);
                    return (Statement::VarDecl(VarDecl { var_id, expression }), None);
                }
            }
        }
        (
            Statement::Expression(Expression {
                expr: Expr::Garbage,
                span: span(spans),
            }),
            Some(ParseError::UnknownState(
                "internal error: let statement unparseable".into(),
                span(spans),
            )),
        )
    }

    pub fn parse_statement(&mut self, spans: &[Span]) -> (Statement, Option<ParseError>) {
        if let (stmt, None) = self.parse_let(spans) {
            (stmt, None)
        } else {
            let (expr, err) = self.parse_expression(spans);
            (Statement::Expression(expr), err)
        }
    }

    pub fn parse_block(&mut self, lite_block: &LiteBlock) -> (Block, Option<ParseError>) {
        let mut error = None;
        self.enter_scope();

        let mut block = Block::new();

        for pipeline in &lite_block.block {
            let (stmt, err) = self.parse_statement(&pipeline.commands[0].parts);
            error = error.or(err);

            block.stmts.push(stmt);
        }

        self.exit_scope();

        (block, error)
    }

    pub fn parse_file(&mut self, fname: &str, contents: Vec<u8>) -> (Block, Option<ParseError>) {
        let mut error = None;

        let (output, err) = lex(&contents, 0, crate::LexMode::Normal);
        error = error.or(err);

        self.add_file(fname.into(), contents);

        let (output, err) = lite_parse(&output);
        error = error.or(err);

        let (output, err) = self.parse_block(&output);
        error = error.or(err);

        (output, error)
    }

    pub fn parse_source(&mut self, source: &[u8]) -> (Block, Option<ParseError>) {
        let mut error = None;

        self.add_file("source".into(), source.into());

        let (output, err) = lex(source, 0, crate::LexMode::Normal);
        error = error.or(err);

        let (output, err) = lite_parse(&output);
        error = error.or(err);

        let (output, err) = self.parse_block(&output);
        error = error.or(err);

        (output, error)
    }
}

#[cfg(test)]
mod tests {
    use crate::{ParseError, Signature};

    use super::*;

    #[test]
    pub fn parse_int() {
        let mut working_set = ParserWorkingSet::new(None);

        let (block, err) = working_set.parse_source(b"3");

        assert!(err.is_none());
        assert!(block.len() == 1);
        assert!(matches!(
            block[0],
            Statement::Expression(Expression {
                expr: Expr::Int(3),
                ..
            })
        ));
    }

    #[test]
    pub fn parse_call() {
        let mut working_set = ParserWorkingSet::new(None);

        let sig = Signature::build("foo").named("--jazz", SyntaxShape::Int, "jazz!!", Some('j'));
        working_set.add_decl((b"foo").to_vec(), sig);

        let (block, err) = working_set.parse_source(b"foo");

        assert!(err.is_none());
        assert!(block.len() == 1);

        match &block[0] {
            Statement::Expression(Expression {
                expr: Expr::Call(call),
                ..
            }) => {
                assert_eq!(call.decl_id, 0);
            }
            _ => panic!("not a call"),
        }
    }

    #[test]
    pub fn parse_call_missing_flag_arg() {
        let mut working_set = ParserWorkingSet::new(None);

        let sig = Signature::build("foo").named("--jazz", SyntaxShape::Int, "jazz!!", Some('j'));
        working_set.add_decl((b"foo").to_vec(), sig);

        let (_, err) = working_set.parse_source(b"foo --jazz");
        assert!(matches!(err, Some(ParseError::MissingFlagParam(..))));
    }

    #[test]
    pub fn parse_call_missing_short_flag_arg() {
        let mut working_set = ParserWorkingSet::new(None);

        let sig = Signature::build("foo").named("--jazz", SyntaxShape::Int, "jazz!!", Some('j'));
        working_set.add_decl((b"foo").to_vec(), sig);

        let (_, err) = working_set.parse_source(b"foo -j");
        assert!(matches!(err, Some(ParseError::MissingFlagParam(..))));
    }

    #[test]
    pub fn parse_call_too_many_shortflag_args() {
        let mut working_set = ParserWorkingSet::new(None);

        let sig = Signature::build("foo")
            .named("--jazz", SyntaxShape::Int, "jazz!!", Some('j'))
            .named("--math", SyntaxShape::Int, "math!!", Some('m'));
        working_set.add_decl((b"foo").to_vec(), sig);
        let (_, err) = working_set.parse_source(b"foo -mj");
        assert!(matches!(
            err,
            Some(ParseError::ShortFlagBatchCantTakeArg(..))
        ));
    }

    #[test]
    pub fn parse_call_unknown_shorthand() {
        let mut working_set = ParserWorkingSet::new(None);

        let sig = Signature::build("foo").switch("--jazz", "jazz!!", Some('j'));
        working_set.add_decl((b"foo").to_vec(), sig);
        let (_, err) = working_set.parse_source(b"foo -mj");
        assert!(matches!(err, Some(ParseError::UnknownFlag(..))));
    }

    #[test]
    pub fn parse_call_extra_positional() {
        let mut working_set = ParserWorkingSet::new(None);

        let sig = Signature::build("foo").switch("--jazz", "jazz!!", Some('j'));
        working_set.add_decl((b"foo").to_vec(), sig);
        let (_, err) = working_set.parse_source(b"foo -j 100");
        assert!(matches!(err, Some(ParseError::ExtraPositional(..))));
    }

    #[test]
    pub fn parse_call_missing_req_positional() {
        let mut working_set = ParserWorkingSet::new(None);

        let sig = Signature::build("foo").required("jazz", SyntaxShape::Int, "jazz!!");
        working_set.add_decl((b"foo").to_vec(), sig);
        let (_, err) = working_set.parse_source(b"foo");
        assert!(matches!(err, Some(ParseError::MissingPositional(..))));
    }

    #[test]
    pub fn parse_call_missing_req_flag() {
        let mut working_set = ParserWorkingSet::new(None);

        let sig =
            Signature::build("foo").required_named("--jazz", SyntaxShape::Int, "jazz!!", None);
        working_set.add_decl((b"foo").to_vec(), sig);
        let (_, err) = working_set.parse_source(b"foo");
        assert!(matches!(err, Some(ParseError::MissingRequiredFlag(..))));
    }
}
