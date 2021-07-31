use std::ops::{Index, IndexMut};

use crate::{
    lex, lite_parse,
    parser_state::{Type, VarId},
    signature::{Flag, PositionalArg},
    BlockId, DeclId, Declaration, LiteBlock, ParseError, ParserWorkingSet, Signature, Span, Token,
};

/// The syntactic shapes that values must match to be passed into a command. You can think of this as the type-checking that occurs when you call a function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyntaxShape {
    /// A specific match to a word or symbol
    Keyword(Vec<u8>, Box<SyntaxShape>),

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

    /// A table is allowed, eg `[[first, second]; [1, 2]]`
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

    /// A variable with optional type, `x` or `x: int`
    VarWithOptType,

    /// A signature for a definition, `[x:int, --foo]`
    Signature,

    /// A general expression, eg `1 + 2` or `foo --bar`
    Expression,
}

impl SyntaxShape {
    pub fn to_type(&self) -> Type {
        match self {
            SyntaxShape::Any => Type::Unknown,
            SyntaxShape::Block => Type::Block,
            SyntaxShape::ColumnPath => Type::Unknown,
            SyntaxShape::Duration => Type::Duration,
            SyntaxShape::Expression => Type::Unknown,
            SyntaxShape::FilePath => Type::FilePath,
            SyntaxShape::Filesize => Type::Filesize,
            SyntaxShape::FullColumnPath => Type::Unknown,
            SyntaxShape::GlobPattern => Type::String,
            SyntaxShape::Int => Type::Int,
            SyntaxShape::List(x) => {
                let contents = x.to_type();
                Type::List(Box::new(contents))
            }
            SyntaxShape::Keyword(_, expr) => expr.to_type(),
            SyntaxShape::MathExpression => Type::Unknown,
            SyntaxShape::Number => Type::Number,
            SyntaxShape::Operator => Type::Unknown,
            SyntaxShape::Range => Type::Unknown,
            SyntaxShape::RowCondition => Type::Bool,
            SyntaxShape::Signature => Type::Unknown,
            SyntaxShape::String => Type::String,
            SyntaxShape::Table => Type::Table,
            SyntaxShape::VarWithOptType => Type::Unknown,
            SyntaxShape::Variable => Type::Unknown,
        }
    }
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
    pub head: Span,
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
            head: Span::unknown(),
            positional: vec![],
            named: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub enum Expr {
    Bool(bool),
    Int(i64),
    Var(VarId),
    Call(Box<Call>),
    ExternalCall(Vec<u8>, Vec<Vec<u8>>),
    Operator(Operator),
    BinaryOp(Box<Expression>, Box<Expression>, Box<Expression>), //lhs, op, rhs
    Subexpression(BlockId),
    Block(BlockId),
    List(Vec<Expression>),
    Table(Vec<Expression>, Vec<Vec<Expression>>),
    Keyword(Vec<u8>, Span, Box<Expression>),
    String(String), // FIXME: improve this in the future?
    Signature(Box<Signature>),
    Garbage,
}

#[derive(Debug, Clone)]
pub struct Expression {
    pub expr: Expr,
    pub span: Span,
    pub ty: Type,
}
impl Expression {
    pub fn garbage(span: Span) -> Expression {
        Expression {
            expr: Expr::Garbage,
            span,
            ty: Type::Unknown,
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

    pub fn as_block(&self) -> Option<BlockId> {
        match self.expr {
            Expr::Block(block_id) => Some(block_id),
            _ => None,
        }
    }

    pub fn as_signature(&self) -> Option<Box<Signature>> {
        match &self.expr {
            Expr::Signature(sig) => Some(sig.clone()),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<Vec<Expression>> {
        match &self.expr {
            Expr::List(list) => Some(list.clone()),
            _ => None,
        }
    }

    pub fn as_keyword(&self) -> Option<&Expression> {
        match &self.expr {
            Expr::Keyword(_, _, expr) => Some(expr),
            _ => None,
        }
    }

    pub fn as_var(&self) -> Option<VarId> {
        match self.expr {
            Expr::Var(var_id) => Some(var_id),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<String> {
        match &self.expr {
            Expr::String(string) => Some(string.clone()),
            _ => None,
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
    Declaration(DeclId),
    Pipeline(Pipeline),
    Expression(Expression),
}

#[derive(Debug, Clone)]
pub struct Pipeline {
    pub expressions: Vec<Expression>,
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl Pipeline {
    pub fn new() -> Self {
        Self {
            expressions: vec![],
        }
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

impl<'a> ParserWorkingSet<'a> {
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
                ty: Type::Unknown,
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

        if arg_contents.starts_with(b"--") {
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
                            let (arg, err) = self.parse_value(span, arg_shape);

                            (Some(long_name), Some(arg), err)
                        } else if let Some(arg) = spans.get(*spans_idx + 1) {
                            let (arg, err) = self.parse_value(*arg, arg_shape);

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

        if arg_contents.starts_with(b"-") && arg_contents.len() > 1 {
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
                        if String::from_utf8_lossy(arg_contents).parse::<f64>().is_ok() {
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

    fn calculate_end_span(
        &self,
        decl: &Declaration,
        spans: &[Span],
        spans_idx: usize,
        positional_idx: usize,
    ) -> usize {
        if decl.signature.rest_positional.is_some() {
            spans.len()
        } else {
            // println!("num_positionals: {}", decl.signature.num_positionals());
            // println!("positional_idx: {}", positional_idx);
            // println!("spans.len(): {}", spans.len());
            // println!("spans_idx: {}", spans_idx);

            // check to see if a keyword follows the current position.

            let mut next_keyword_idx = spans.len();
            for idx in (positional_idx + 1)..decl.signature.num_positionals() {
                if let Some(PositionalArg {
                    shape: SyntaxShape::Keyword(kw, ..),
                    ..
                }) = decl.signature.get_positional(idx)
                {
                    #[allow(clippy::needless_range_loop)]
                    for span_idx in spans_idx..spans.len() {
                        let contents = self.get_span_contents(spans[span_idx]);

                        if contents == kw {
                            next_keyword_idx = span_idx - (idx - (positional_idx + 1));
                            break;
                        }
                    }
                }
            }

            let remainder = decl.signature.num_positionals_after(positional_idx);
            let remainder_idx = if remainder < spans.len() {
                spans.len() - remainder + 1
            } else {
                spans_idx + 1
            };

            let end = [next_keyword_idx, remainder_idx, spans.len()]
                .iter()
                .min()
                .copied()
                .expect("internal error: can't find min");

            // println!(
            //     "{:?}",
            //     [
            //         next_keyword_idx,
            //         remainder_idx,
            //         spans.len(),
            //         spans_idx,
            //         remainder,
            //         positional_idx,
            //     ]
            // );
            end
        }
    }

    fn parse_multispan_value(
        &mut self,
        spans: &[Span],
        spans_idx: &mut usize,
        shape: &SyntaxShape,
    ) -> (Expression, Option<ParseError>) {
        let mut error = None;

        match shape {
            SyntaxShape::VarWithOptType => {
                let (arg, err) = self.parse_var_with_opt_type(spans, spans_idx);
                error = error.or(err);

                (arg, error)
            }
            SyntaxShape::RowCondition => {
                let (arg, err) = self.parse_row_condition(&spans[*spans_idx..]);
                error = error.or(err);
                *spans_idx = spans.len() - 1;

                (arg, error)
            }
            SyntaxShape::Expression => {
                let (arg, err) = self.parse_expression(&spans[*spans_idx..]);
                error = error.or(err);
                *spans_idx = spans.len() - 1;

                (arg, error)
            }
            SyntaxShape::Keyword(keyword, arg) => {
                let arg_span = spans[*spans_idx];

                let arg_contents = self.get_span_contents(arg_span);

                if arg_contents != keyword {
                    // When keywords mismatch, this is a strong indicator of something going wrong.
                    // We won't often override the current error, but as this is a strong indicator
                    // go ahead and override the current error and tell the user about the missing
                    // keyword/literal.
                    error = Some(ParseError::Mismatch(
                        String::from_utf8_lossy(keyword).into(),
                        arg_span,
                    ))
                }

                *spans_idx += 1;
                if *spans_idx >= spans.len() {
                    error = error.or_else(|| {
                        Some(ParseError::MissingPositional(
                            String::from_utf8_lossy(keyword).into(),
                            spans[*spans_idx - 1],
                        ))
                    });
                    return (
                        Expression {
                            expr: Expr::Keyword(
                                keyword.clone(),
                                spans[*spans_idx - 1],
                                Box::new(Expression::garbage(arg_span)),
                            ),
                            span: arg_span,
                            ty: Type::Unknown,
                        },
                        error,
                    );
                }
                let keyword_span = spans[*spans_idx - 1];
                let (expr, err) = self.parse_multispan_value(spans, spans_idx, arg);
                error = error.or(err);
                let ty = expr.ty.clone();

                (
                    Expression {
                        expr: Expr::Keyword(keyword.clone(), keyword_span, Box::new(expr)),
                        span: arg_span,
                        ty,
                    },
                    error,
                )
            }
            _ => {
                // All other cases are single-span values
                let arg_span = spans[*spans_idx];

                let (arg, err) = self.parse_value(arg_span, shape);
                error = error.or(err);

                (arg, error)
            }
        }
    }

    pub fn parse_internal_call(
        &mut self,
        command_span: Span,
        spans: &[Span],
        decl_id: usize,
    ) -> (Box<Call>, Span, Option<ParseError>) {
        let mut error = None;

        let mut call = Call::new();
        call.decl_id = decl_id;
        call.head = command_span;

        let decl = self.get_decl(decl_id).clone();

        // The index into the positional parameter in the definition
        let mut positional_idx = 0;

        // The index into the spans of argument data given to parse
        // Starting at the first argument
        let mut spans_idx = 0;

        while spans_idx < spans.len() {
            let arg_span = spans[spans_idx];

            // Check if we're on a long flag, if so, parse
            let (long_name, arg, err) =
                self.parse_long_flag(spans, &mut spans_idx, &decl.signature);
            if let Some(long_name) = long_name {
                // We found a long flag, like --bar
                error = error.or(err);
                call.named.push((long_name, arg));
                spans_idx += 1;
                continue;
            }

            // Check if we're on a short flag or group of short flags, if so, parse
            let (short_flags, err) =
                self.parse_short_flags(spans, &mut spans_idx, positional_idx, &decl.signature);

            if let Some(short_flags) = short_flags {
                error = error.or(err);
                for flag in short_flags {
                    if let Some(arg_shape) = flag.arg {
                        if let Some(arg) = spans.get(spans_idx + 1) {
                            let (arg, err) = self.parse_value(*arg, &arg_shape);
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

            // Parse a positional arg if there is one
            if let Some(positional) = decl.signature.get_positional(positional_idx) {
                //Make sure we leave enough spans for the remaining positionals

                let end = self.calculate_end_span(&decl, spans, spans_idx, positional_idx);

                let orig_idx = spans_idx;
                let (arg, err) =
                    self.parse_multispan_value(&spans[..end], &mut spans_idx, &positional.shape);
                error = error.or(err);

                let arg = if positional.shape.to_type() != Type::Unknown
                    && arg.ty != positional.shape.to_type()
                {
                    let span = span(&spans[orig_idx..spans_idx]);
                    error = error.or_else(|| {
                        Some(ParseError::TypeMismatch(positional.shape.to_type(), span))
                    });
                    Expression::garbage(span)
                } else {
                    arg
                };
                call.positional.push(arg);
                positional_idx += 1;
            } else {
                call.positional.push(Expression::garbage(arg_span));
                error = error.or(Some(ParseError::ExtraPositional(arg_span)))
            }

            error = error.or(err);
            spans_idx += 1;
        }

        let err = check_call(command_span, &decl.signature, &call);
        error = error.or(err);

        // FIXME: type unknown
        (Box::new(call), span(spans), error)
    }

    pub fn parse_call(&mut self, spans: &[Span]) -> (Expression, Option<ParseError>) {
        // assume spans.len() > 0?
        let mut pos = 0;
        let mut shorthand = vec![];

        while pos < spans.len() {
            // First, check if there is any environment shorthand
            let name = self.get_span_contents(spans[pos]);
            let split: Vec<_> = name.splitn(2, |x| *x == b'=').collect();
            if split.len() == 2 {
                shorthand.push(split);
                pos += 1;
            } else {
                break;
            }
        }

        if pos == spans.len() {
            return (
                Expression::garbage(span(spans)),
                Some(ParseError::UnknownCommand(spans[0])),
            );
        }
        let name = self.get_span_contents(spans[pos]);
        pos += 1;

        if let Some(mut decl_id) = self.find_decl(name) {
            let mut name = name.to_vec();
            while pos < spans.len() {
                // look to see if it's a subcommand
                let mut new_name = name.to_vec();
                new_name.push(b' ');
                new_name.extend(self.get_span_contents(spans[pos]));

                if let Some(did) = self.find_decl(&new_name) {
                    decl_id = did;
                } else {
                    break;
                }
                name = new_name;
                pos += 1;
            }
            // parse internal command
            let (call, _, err) =
                self.parse_internal_call(span(&spans[0..pos]), &spans[pos..], decl_id);
            (
                Expression {
                    expr: Expr::Call(call),
                    span: span(spans),
                    ty: Type::Unknown, // FIXME
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
                        ty: Type::Int,
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
                        ty: Type::Int,
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
                        ty: Type::Int,
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
                    ty: Type::Int,
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
        let contents = self.get_span_contents(span);

        if contents.starts_with(b"$\"") {
            self.parse_string_interpolation(span)
        } else {
            self.parse_variable_expr(span)
        }
    }

    pub fn parse_string_interpolation(&mut self, span: Span) -> (Expression, Option<ParseError>) {
        #[derive(PartialEq, Eq, Debug)]
        enum InterpolationMode {
            String,
            Expression,
        }
        let mut error = None;

        let contents = self.get_span_contents(span);

        let start = if contents.starts_with(b"$\"") {
            span.start + 2
        } else {
            span.start
        };

        let end = if contents.ends_with(b"\"") && contents.len() > 2 {
            span.end - 1
        } else {
            span.end
        };

        let inner_span = Span { start, end };
        let contents = self.get_span_contents(inner_span).to_vec();

        let mut output = vec![];
        let mut mode = InterpolationMode::String;
        let mut token_start = start;
        let mut depth = 0;

        let mut b = start;

        #[allow(clippy::needless_range_loop)]
        while b != end {
            if contents[b - start] == b'(' && mode == InterpolationMode::String {
                depth = 1;
                mode = InterpolationMode::Expression;
                if token_start < b {
                    let span = Span {
                        start: token_start,
                        end: b,
                    };
                    let str_contents = self.get_span_contents(span);
                    output.push(Expression {
                        expr: Expr::String(String::from_utf8_lossy(str_contents).to_string()),
                        span,
                        ty: Type::String,
                    });
                }
                token_start = b;
            } else if contents[b - start] == b'(' && mode == InterpolationMode::Expression {
                depth += 1;
            } else if contents[b - start] == b')' && mode == InterpolationMode::Expression {
                match depth {
                    0 => {}
                    1 => {
                        mode = InterpolationMode::String;

                        if token_start < b {
                            let span = Span {
                                start: token_start,
                                end: b + 1,
                            };

                            let (expr, err) = self.parse_full_column_path(span);
                            error = error.or(err);
                            output.push(expr);
                        }

                        token_start = b + 1;
                    }
                    _ => depth -= 1,
                }
            }
            b += 1;
        }

        match mode {
            InterpolationMode::String => {
                if token_start < end {
                    let span = Span {
                        start: token_start,
                        end,
                    };
                    let str_contents = self.get_span_contents(span);
                    output.push(Expression {
                        expr: Expr::String(String::from_utf8_lossy(str_contents).to_string()),
                        span,
                        ty: Type::String,
                    });
                }
            }
            InterpolationMode::Expression => {
                if token_start < end {
                    let span = Span {
                        start: token_start,
                        end,
                    };

                    let (expr, err) = self.parse_full_column_path(span);
                    error = error.or(err);
                    output.push(expr);
                }
            }
        }

        if let Some(decl_id) = self.find_decl(b"build-string") {
            (
                Expression {
                    expr: Expr::Call(Box::new(Call {
                        head: Span {
                            start: span.start,
                            end: span.start + 2,
                        },
                        named: vec![],
                        positional: output,
                        decl_id,
                    })),
                    span,
                    ty: Type::String,
                },
                error,
            )
        } else {
            (
                Expression::garbage(span),
                Some(ParseError::UnknownCommand(span)),
            )
        }
    }

    pub fn parse_variable_expr(&mut self, span: Span) -> (Expression, Option<ParseError>) {
        let contents = self.get_span_contents(span);

        if contents == b"$true" {
            return (
                Expression {
                    expr: Expr::Bool(true),
                    span,
                    ty: Type::Bool,
                },
                None,
            );
        } else if contents == b"$false" {
            return (
                Expression {
                    expr: Expr::Bool(false),
                    span,
                    ty: Type::Bool,
                },
                None,
            );
        }

        let (id, err) = self.parse_variable(span);

        if err.is_none() {
            if let Some(id) = id {
                (
                    Expression {
                        expr: Expr::Var(id),
                        span,
                        ty: self.get_variable(id).clone(),
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
                        ty: Type::Unknown,
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

        let (output, err) = lex(source, start, &[], &[]);
        error = error.or(err);

        let (output, err) = lite_parse(&output);
        error = error.or(err);

        let (output, err) = self.parse_block(&output, true);
        error = error.or(err);

        let block_id = self.add_block(output);

        (
            Expression {
                expr: Expr::Subexpression(block_id),
                span,
                ty: Type::Unknown, // FIXME
            },
            error,
        )
    }

    pub fn parse_string(&mut self, span: Span) -> (Expression, Option<ParseError>) {
        let bytes = self.get_span_contents(span);
        let bytes = if (bytes.starts_with(b"\"") && bytes.ends_with(b"\"") && bytes.len() > 1)
            || (bytes.starts_with(b"\'") && bytes.ends_with(b"\'") && bytes.len() > 1)
        {
            &bytes[1..(bytes.len() - 1)]
        } else {
            bytes
        };

        if let Ok(token) = String::from_utf8(bytes.into()) {
            (
                Expression {
                    expr: Expr::String(token),
                    span,
                    ty: Type::String,
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

    //TODO: Handle error case
    pub fn parse_shape_name(&self, bytes: &[u8], span: Span) -> (SyntaxShape, Option<ParseError>) {
        let result = match bytes {
            b"any" => SyntaxShape::Any,
            b"string" => SyntaxShape::String,
            b"column-path" => SyntaxShape::ColumnPath,
            b"number" => SyntaxShape::Number,
            b"range" => SyntaxShape::Range,
            b"int" => SyntaxShape::Int,
            b"path" => SyntaxShape::FilePath,
            b"glob" => SyntaxShape::GlobPattern,
            b"block" => SyntaxShape::Block,
            b"cond" => SyntaxShape::RowCondition,
            b"operator" => SyntaxShape::Operator,
            b"math" => SyntaxShape::MathExpression,
            b"variable" => SyntaxShape::Variable,
            b"signature" => SyntaxShape::Signature,
            b"expr" => SyntaxShape::Expression,
            _ => return (SyntaxShape::Any, Some(ParseError::UnknownType(span))),
        };

        (result, None)
    }

    pub fn parse_type(&self, bytes: &[u8]) -> Type {
        if bytes == b"int" {
            Type::Int
        } else {
            Type::Unknown
        }
    }

    pub fn parse_var_with_opt_type(
        &mut self,
        spans: &[Span],
        spans_idx: &mut usize,
    ) -> (Expression, Option<ParseError>) {
        let bytes = self.get_span_contents(spans[*spans_idx]).to_vec();

        if bytes.ends_with(b":") {
            // We end with colon, so the next span should be the type
            if *spans_idx + 1 < spans.len() {
                *spans_idx += 1;
                let type_bytes = self.get_span_contents(spans[*spans_idx]);

                let ty = self.parse_type(type_bytes);

                let id = self.add_variable(bytes[0..(bytes.len() - 1)].to_vec(), ty.clone());

                (
                    Expression {
                        expr: Expr::Var(id),
                        span: span(&spans[*spans_idx - 1..*spans_idx + 1]),
                        ty,
                    },
                    None,
                )
            } else {
                let id = self.add_variable(bytes[0..(bytes.len() - 1)].to_vec(), Type::Unknown);
                (
                    Expression {
                        expr: Expr::Var(id),
                        span: spans[*spans_idx],
                        ty: Type::Unknown,
                    },
                    Some(ParseError::MissingType(spans[*spans_idx])),
                )
            }
        } else {
            let id = self.add_variable(bytes, Type::Unknown);

            (
                Expression {
                    expr: Expr::Var(id),
                    span: span(&spans[*spans_idx..*spans_idx + 1]),
                    ty: Type::Unknown,
                },
                None,
            )
        }
    }
    pub fn parse_row_condition(&mut self, spans: &[Span]) -> (Expression, Option<ParseError>) {
        self.parse_math_expression(spans)
    }

    pub fn parse_signature(&mut self, span: Span) -> (Expression, Option<ParseError>) {
        enum ParseMode {
            ArgMode,
            TypeMode,
        }

        enum Arg {
            Positional(PositionalArg, bool), // bool - required
            Flag(Flag),
        }

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
        let source = self.get_span_contents(span);

        let (output, err) = lex(source, span.start, &[b'\n', b','], &[b':']);
        error = error.or(err);

        let mut args: Vec<Arg> = vec![];
        let mut parse_mode = ParseMode::ArgMode;

        for token in &output {
            match token {
                Token {
                    contents: crate::TokenContents::Item,
                    span,
                } => {
                    let span = *span;
                    let contents = self.get_span_contents(span);

                    if contents == b":" {
                        match parse_mode {
                            ParseMode::ArgMode => {
                                parse_mode = ParseMode::TypeMode;
                            }
                            ParseMode::TypeMode => {
                                // We're seeing two types for the same thing for some reason, error
                                error = error
                                    .or_else(|| Some(ParseError::Mismatch("type".into(), span)));
                            }
                        }
                    } else {
                        match parse_mode {
                            ParseMode::ArgMode => {
                                if contents.starts_with(b"--") && contents.len() > 2 {
                                    // Long flag
                                    let flags: Vec<_> = contents
                                        .split(|x| x == &b'(')
                                        .map(|x| x.to_vec())
                                        .collect();

                                    let long = String::from_utf8_lossy(&flags[0]).to_string();
                                    let variable_name = flags[0][2..].to_vec();
                                    let var_id = self.add_variable(variable_name, Type::Unknown);

                                    if flags.len() == 1 {
                                        args.push(Arg::Flag(Flag {
                                            arg: None,
                                            desc: String::new(),
                                            long,
                                            short: None,
                                            required: false,
                                            var_id: Some(var_id),
                                        }));
                                    } else {
                                        let short_flag = &flags[1];
                                        let short_flag = if !short_flag.starts_with(b"-")
                                            || !short_flag.ends_with(b")")
                                        {
                                            error = error.or_else(|| {
                                                Some(ParseError::Mismatch(
                                                    "short flag".into(),
                                                    span,
                                                ))
                                            });
                                            short_flag
                                        } else {
                                            &short_flag[1..(short_flag.len() - 1)]
                                        };

                                        let short_flag =
                                            String::from_utf8_lossy(short_flag).to_string();
                                        let chars: Vec<char> = short_flag.chars().collect();
                                        let long = String::from_utf8_lossy(&flags[0]).to_string();
                                        let variable_name = flags[0][2..].to_vec();
                                        let var_id =
                                            self.add_variable(variable_name, Type::Unknown);

                                        if chars.len() == 1 {
                                            args.push(Arg::Flag(Flag {
                                                arg: None,
                                                desc: String::new(),
                                                long,
                                                short: Some(chars[0]),
                                                required: false,
                                                var_id: Some(var_id),
                                            }));
                                        } else {
                                            error = error.or_else(|| {
                                                Some(ParseError::Mismatch(
                                                    "short flag".into(),
                                                    span,
                                                ))
                                            });
                                        }
                                    }
                                } else if contents.starts_with(b"-") && contents.len() > 1 {
                                    // Short flag

                                    let short_flag = &contents[1..];
                                    let short_flag =
                                        String::from_utf8_lossy(short_flag).to_string();
                                    let chars: Vec<char> = short_flag.chars().collect();

                                    if chars.len() > 1 {
                                        error = error.or_else(|| {
                                            Some(ParseError::Mismatch("short flag".into(), span))
                                        });

                                        args.push(Arg::Flag(Flag {
                                            arg: None,
                                            desc: String::new(),
                                            long: String::new(),
                                            short: None,
                                            required: false,
                                            var_id: None,
                                        }));
                                    } else {
                                        let mut encoded_var_name = vec![0u8; 4];
                                        let len = chars[0].encode_utf8(&mut encoded_var_name).len();
                                        let variable_name = encoded_var_name[0..len].to_vec();
                                        let var_id =
                                            self.add_variable(variable_name, Type::Unknown);

                                        args.push(Arg::Flag(Flag {
                                            arg: None,
                                            desc: String::new(),
                                            long: String::new(),
                                            short: Some(chars[0]),
                                            required: false,
                                            var_id: Some(var_id),
                                        }));
                                    }
                                } else if contents.starts_with(b"(-") {
                                    let short_flag = &contents[2..];

                                    let short_flag = if !short_flag.ends_with(b")") {
                                        error = error.or_else(|| {
                                            Some(ParseError::Mismatch("short flag".into(), span))
                                        });
                                        short_flag
                                    } else {
                                        &short_flag[..(short_flag.len() - 1)]
                                    };

                                    let short_flag =
                                        String::from_utf8_lossy(short_flag).to_string();
                                    let chars: Vec<char> = short_flag.chars().collect();

                                    if chars.len() == 1 {
                                        match args.last_mut() {
                                            Some(Arg::Flag(flag)) => {
                                                if flag.short.is_some() {
                                                    error = error.or_else(|| {
                                                        Some(ParseError::Mismatch(
                                                            "one short flag".into(),
                                                            span,
                                                        ))
                                                    });
                                                } else {
                                                    flag.short = Some(chars[0]);
                                                }
                                            }
                                            _ => {
                                                error = error.or_else(|| {
                                                    Some(ParseError::Mismatch(
                                                        "unknown flag".into(),
                                                        span,
                                                    ))
                                                });
                                            }
                                        }
                                    } else {
                                        error = error.or_else(|| {
                                            Some(ParseError::Mismatch("short flag".into(), span))
                                        });
                                    }
                                } else if contents.ends_with(b"?") {
                                    let contents: Vec<_> = contents[..(contents.len() - 1)].into();
                                    let name = String::from_utf8_lossy(&contents).to_string();

                                    let var_id = self.add_variable(contents, Type::Unknown);

                                    // Positional arg, optional
                                    args.push(Arg::Positional(
                                        PositionalArg {
                                            desc: String::new(),
                                            name,
                                            shape: SyntaxShape::Any,
                                            var_id: Some(var_id),
                                        },
                                        false,
                                    ))
                                } else {
                                    let name = String::from_utf8_lossy(contents).to_string();
                                    let contents_vec = contents.to_vec();

                                    let var_id = self.add_variable(contents_vec, Type::Unknown);

                                    // Positional arg, required
                                    args.push(Arg::Positional(
                                        PositionalArg {
                                            desc: String::new(),
                                            name,
                                            shape: SyntaxShape::Any,
                                            var_id: Some(var_id),
                                        },
                                        true,
                                    ))
                                }
                            }
                            ParseMode::TypeMode => {
                                if let Some(last) = args.last_mut() {
                                    let (syntax_shape, err) = self.parse_shape_name(contents, span);
                                    error = error.or(err);
                                    //TODO check if we're replacing one already
                                    match last {
                                        Arg::Positional(
                                            PositionalArg { shape, var_id, .. },
                                            ..,
                                        ) => {
                                            self.set_variable_type(var_id.expect("internal error: all custom parameters must have var_ids"), syntax_shape.to_type());
                                            *shape = syntax_shape;
                                        }
                                        Arg::Flag(Flag { arg, var_id, .. }) => {
                                            self.set_variable_type(var_id.expect("internal error: all custom parameters must have var_ids"), syntax_shape.to_type());
                                            *arg = Some(syntax_shape)
                                        }
                                    }
                                }
                                parse_mode = ParseMode::ArgMode;
                            }
                        }
                    }
                }
                Token {
                    contents: crate::TokenContents::Comment,
                    span,
                } => {
                    let contents = self.get_span_contents(Span {
                        start: span.start + 1,
                        end: span.end,
                    });

                    let mut contents = String::from_utf8_lossy(contents).to_string();
                    contents = contents.trim().into();

                    if let Some(last) = args.last_mut() {
                        match last {
                            Arg::Flag(flag) => {
                                if !flag.desc.is_empty() {
                                    flag.desc.push('\n');
                                }
                                flag.desc.push_str(&contents);
                            }
                            Arg::Positional(positional, ..) => {
                                if !positional.desc.is_empty() {
                                    positional.desc.push('\n');
                                }
                                positional.desc.push_str(&contents);
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        let mut sig = Signature::new(String::new());

        for arg in args {
            match arg {
                Arg::Positional(positional, required) => {
                    if positional.name == "...rest" {
                        if sig.rest_positional.is_none() {
                            sig.rest_positional = Some(PositionalArg {
                                name: "rest".into(),
                                ..positional
                            })
                        } else {
                            // Too many rest params
                            error = error.or(Some(ParseError::MultipleRestParams(span)))
                        }
                    } else if required {
                        sig.required_positional.push(positional)
                    } else {
                        sig.optional_positional.push(positional)
                    }
                }
                Arg::Flag(flag) => sig.named.push(flag),
            }
        }

        (
            Expression {
                expr: Expr::Signature(Box::new(sig)),
                span,
                ty: Type::Unknown,
            },
            error,
        )
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
        let source = self.get_span_contents(span);

        let (output, err) = lex(source, span.start, &[b'\n', b','], &[]);
        error = error.or(err);

        let (output, err) = lite_parse(&output);
        error = error.or(err);

        let mut args = vec![];

        if !output.block.is_empty() {
            for arg in &output.block[0].commands {
                let mut spans_idx = 0;

                while spans_idx < arg.parts.len() {
                    let (arg, err) =
                        self.parse_multispan_value(&arg.parts, &mut spans_idx, element_shape);
                    error = error.or(err);

                    args.push(arg);

                    spans_idx += 1;
                }
            }
        }

        (
            Expression {
                expr: Expr::List(args),
                span,
                ty: Type::List(Box::new(Type::Unknown)), // FIXME
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

        let source = self.get_span_contents(span);

        let (output, err) = lex(source, start, &[b'\n', b','], &[]);
        error = error.or(err);

        let (output, err) = lite_parse(&output);
        error = error.or(err);

        match output.block.len() {
            0 => (
                Expression {
                    expr: Expr::List(vec![]),
                    span,
                    ty: Type::Table,
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
                    self.parse_value(output.block[0].commands[0].parts[0], &SyntaxShape::Table);
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
                    let (values, err) = self.parse_value(*part, &SyntaxShape::Table);
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
                        ty: Type::Table,
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

        let source = self.get_span_contents(span);

        let (output, err) = lex(source, start, &[], &[]);
        error = error.or(err);

        let (output, err) = lite_parse(&output);
        error = error.or(err);

        let (output, err) = self.parse_block(&output, true);
        error = error.or(err);

        let block_id = self.add_block(output);

        (
            Expression {
                expr: Expr::Block(block_id),
                span,
                ty: Type::Block,
            },
            error,
        )
    }

    pub fn parse_value(
        &mut self,
        span: Span,
        shape: &SyntaxShape,
    ) -> (Expression, Option<ParseError>) {
        let bytes = self.get_span_contents(span);

        // First, check the special-cases. These will likely represent specific values as expressions
        // and may fit a variety of shapes.
        //
        // We check variable first because immediately following we check for variables with column paths
        // which might result in a value that fits other shapes (and require the variable to already be
        // declared)
        if shape == &SyntaxShape::Variable {
            return self.parse_variable_expr(span);
        } else if bytes.starts_with(b"$") {
            return self.parse_dollar_expr(span);
        } else if bytes.starts_with(b"(") {
            return self.parse_full_column_path(span);
        } else if bytes.starts_with(b"[") {
            match shape {
                SyntaxShape::Any
                | SyntaxShape::List(_)
                | SyntaxShape::Table
                | SyntaxShape::Signature => {}
                _ => {
                    return (
                        Expression::garbage(span),
                        Some(ParseError::Mismatch("non-[] value".into(), span)),
                    );
                }
            }
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
            SyntaxShape::Signature => {
                if bytes.starts_with(b"[") {
                    self.parse_signature(span)
                } else {
                    (
                        Expression::garbage(span),
                        Some(ParseError::Mismatch("signature".into(), span)),
                    )
                }
            }
            SyntaxShape::List(elem) => {
                if bytes.starts_with(b"[") {
                    self.parse_list_expression(span, elem)
                } else {
                    (
                        Expression::garbage(span),
                        Some(ParseError::Mismatch("list".into(), span)),
                    )
                }
            }
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
                let shapes = [
                    SyntaxShape::Int,
                    SyntaxShape::Number,
                    SyntaxShape::Range,
                    SyntaxShape::Filesize,
                    SyntaxShape::Duration,
                    SyntaxShape::Block,
                    SyntaxShape::Table,
                    SyntaxShape::List(Box::new(SyntaxShape::Any)),
                    SyntaxShape::String,
                ];
                for shape in shapes.iter() {
                    if let (s, None) = self.parse_value(span, shape) {
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
                ty: Type::Unknown,
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
        let (lhs, err) = self.parse_value(spans[0], &SyntaxShape::Any);
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

                expr_stack.push(Expression::garbage(spans[idx - 1]));
                expr_stack.push(Expression::garbage(spans[idx - 1]));

                break;
            }

            let (rhs, err) = self.parse_value(spans[idx], &SyntaxShape::Any);
            error = error.or(err);

            if op_prec <= last_prec {
                while expr_stack.len() > 1 {
                    // Collapse the right associated operations first
                    // so that we can get back to a stack with a lower precedence
                    let mut rhs = expr_stack
                        .pop()
                        .expect("internal error: expression stack empty");
                    let mut op = expr_stack
                        .pop()
                        .expect("internal error: expression stack empty");
                    let mut lhs = expr_stack
                        .pop()
                        .expect("internal error: expression stack empty");

                    let (result_ty, err) = self.math_result_type(&mut lhs, &mut op, &mut rhs);
                    error = error.or(err);

                    let op_span = span(&[lhs.span, rhs.span]);
                    expr_stack.push(Expression {
                        expr: Expr::BinaryOp(Box::new(lhs), Box::new(op), Box::new(rhs)),
                        span: op_span,
                        ty: result_ty,
                    });
                }
            }
            expr_stack.push(op);
            expr_stack.push(rhs);

            last_prec = op_prec;

            idx += 1;
        }

        while expr_stack.len() != 1 {
            let mut rhs = expr_stack
                .pop()
                .expect("internal error: expression stack empty");
            let mut op = expr_stack
                .pop()
                .expect("internal error: expression stack empty");
            let mut lhs = expr_stack
                .pop()
                .expect("internal error: expression stack empty");

            let (result_ty, err) = self.math_result_type(&mut lhs, &mut op, &mut rhs);
            error = error.or(err);

            let binary_op_span = span(&[lhs.span, rhs.span]);
            expr_stack.push(Expression {
                expr: Expr::BinaryOp(Box::new(lhs), Box::new(op), Box::new(rhs)),
                span: binary_op_span,
                ty: result_ty,
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
            | b'[' | b'$' | b'"' | b'\'' => self.parse_math_expression(spans),
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

    pub fn parse_def_predecl(&mut self, spans: &[Span]) {
        let name = self.get_span_contents(spans[0]);

        if name == b"def" && spans.len() >= 4 {
            //FIXME: don't use expect here
            let (name_expr, ..) = self.parse_string(spans[1]);
            let name = name_expr
                .as_string()
                .expect("internal error: expected def name");

            self.enter_scope();
            // FIXME: because parse_signature will update the scope with the variables it sees
            // we end up parsing the signature twice per def. The first time is during the predecl
            // so that we can see the types that are part of the signature, which we need for parsing.
            // The second time is when we actually parse the body itself.
            // We can't reuse the first time because the variables that are created during parse_signature
            // are lost when we exit the scope below.
            let (sig, ..) = self.parse_signature(spans[2]);
            let mut signature = sig
                .as_signature()
                .expect("internal error: expected param list");
            self.exit_scope();

            signature.name = name;
            let decl = Declaration {
                signature,
                body: None,
            };

            self.add_decl(decl);
        }
    }

    pub fn parse_def(&mut self, spans: &[Span]) -> (Statement, Option<ParseError>) {
        let mut error = None;
        let name = self.get_span_contents(spans[0]);

        if name == b"def" && spans.len() >= 4 {
            //FIXME: don't use expect here
            let (name_expr, err) = self.parse_string(spans[1]);
            let name = name_expr
                .as_string()
                .expect("internal error: expected def name");
            error = error.or(err);

            let decl_id = self
                .find_decl(name.as_bytes())
                .expect("internal error: predeclaration failed to add definition");

            self.enter_scope();
            let (sig, err) = self.parse_signature(spans[2]);
            let mut signature = sig
                .as_signature()
                .expect("internal error: expected param list");
            signature.name = name;
            error = error.or(err);

            let (block, err) = self.parse_block_expression(spans[3]);
            self.exit_scope();

            let block_id = block.as_block().expect("internal error: expected block");
            error = error.or(err);

            let declaration = self.get_decl_mut(decl_id);
            declaration.signature = signature;
            declaration.body = Some(block_id);

            let def_decl_id = self
                .find_decl(b"def")
                .expect("internal error: missing def command");

            let call = Box::new(Call {
                head: spans[0],
                decl_id: def_decl_id,
                positional: vec![name_expr, sig, block],
                named: vec![],
            });

            (
                Statement::Expression(Expression {
                    expr: Expr::Call(call),
                    span: span(spans),
                    ty: Type::Unknown,
                }),
                error,
            )
        } else {
            (
                Statement::Expression(Expression {
                    expr: Expr::Garbage,
                    span: span(spans),
                    ty: Type::Unknown,
                }),
                Some(ParseError::UnknownState(
                    "internal error: let statement unparseable".into(),
                    span(spans),
                )),
            )
        }
    }

    pub fn parse_let(&mut self, spans: &[Span]) -> (Statement, Option<ParseError>) {
        let name = self.get_span_contents(spans[0]);

        if name == b"let" {
            if let Some(decl_id) = self.find_decl(b"let") {
                let (call, call_span, err) =
                    self.parse_internal_call(spans[0], &spans[1..], decl_id);

                return (
                    Statement::Expression(Expression {
                        expr: Expr::Call(call),
                        span: call_span,
                        ty: Type::Unknown,
                    }),
                    err,
                );
            }
        }
        (
            Statement::Expression(Expression {
                expr: Expr::Garbage,
                span: span(spans),
                ty: Type::Unknown,
            }),
            Some(ParseError::UnknownState(
                "internal error: let statement unparseable".into(),
                span(spans),
            )),
        )
    }

    pub fn parse_statement(&mut self, spans: &[Span]) -> (Statement, Option<ParseError>) {
        // FIXME: improve errors by checking keyword first
        if let (decl, None) = self.parse_def(spans) {
            (decl, None)
        } else if let (stmt, None) = self.parse_let(spans) {
            (stmt, None)
        } else {
            let (expr, err) = self.parse_expression(spans);
            (Statement::Expression(expr), err)
        }
    }

    pub fn parse_block(
        &mut self,
        lite_block: &LiteBlock,
        scoped: bool,
    ) -> (Block, Option<ParseError>) {
        let mut error = None;
        if scoped {
            self.enter_scope();
        }

        let mut block = Block::new();

        // Pre-declare any definition so that definitions
        // that share the same block can see each other
        for pipeline in &lite_block.block {
            if pipeline.commands.len() == 1 {
                self.parse_def_predecl(&pipeline.commands[0].parts);
            }
        }

        for pipeline in &lite_block.block {
            if pipeline.commands.len() > 1 {
                let mut output = vec![];
                for command in &pipeline.commands {
                    let (expr, err) = self.parse_expression(&command.parts);
                    error = error.or(err);

                    output.push(expr);
                }
                block.stmts.push(Statement::Pipeline(Pipeline {
                    expressions: output,
                }));
            } else {
                let (stmt, err) = self.parse_statement(&pipeline.commands[0].parts);
                error = error.or(err);

                block.stmts.push(stmt);
            }
        }

        if scoped {
            self.exit_scope();
        }

        (block, error)
    }

    pub fn parse_file(
        &mut self,
        fname: &str,
        contents: &[u8],
        scoped: bool,
    ) -> (Block, Option<ParseError>) {
        let mut error = None;

        let span_offset = self.next_span_start();

        self.add_file(fname.into(), contents);

        let (output, err) = lex(contents, span_offset, &[], &[]);
        error = error.or(err);

        let (output, err) = lite_parse(&output);
        error = error.or(err);

        let (output, err) = self.parse_block(&output, scoped);
        error = error.or(err);

        (output, error)
    }

    pub fn parse_source(&mut self, source: &[u8], scoped: bool) -> (Block, Option<ParseError>) {
        let mut error = None;

        let span_offset = self.next_span_start();

        self.add_file("source".into(), source);

        let (output, err) = lex(source, span_offset, &[], &[]);
        error = error.or(err);

        let (output, err) = lite_parse(&output);
        error = error.or(err);

        let (output, err) = self.parse_block(&output, scoped);
        error = error.or(err);

        (output, error)
    }
}

#[cfg(test)]
mod tests {
    use crate::{ParseError, ParserState, Signature};

    use super::*;

    #[test]
    pub fn parse_int() {
        let parser_state = ParserState::new();
        let mut working_set = ParserWorkingSet::new(&parser_state);

        let (block, err) = working_set.parse_source(b"3", true);

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
        let parser_state = ParserState::new();
        let mut working_set = ParserWorkingSet::new(&parser_state);

        let sig = Signature::build("foo").named("--jazz", SyntaxShape::Int, "jazz!!", Some('j'));
        working_set.add_decl(sig.into());

        let (block, err) = working_set.parse_source(b"foo", true);

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
        let parser_state = ParserState::new();
        let mut working_set = ParserWorkingSet::new(&parser_state);

        let sig = Signature::build("foo").named("--jazz", SyntaxShape::Int, "jazz!!", Some('j'));
        working_set.add_decl(sig.into());

        let (_, err) = working_set.parse_source(b"foo --jazz", true);
        assert!(matches!(err, Some(ParseError::MissingFlagParam(..))));
    }

    #[test]
    pub fn parse_call_missing_short_flag_arg() {
        let parser_state = ParserState::new();
        let mut working_set = ParserWorkingSet::new(&parser_state);

        let sig = Signature::build("foo").named("--jazz", SyntaxShape::Int, "jazz!!", Some('j'));
        working_set.add_decl(sig.into());

        let (_, err) = working_set.parse_source(b"foo -j", true);
        assert!(matches!(err, Some(ParseError::MissingFlagParam(..))));
    }

    #[test]
    pub fn parse_call_too_many_shortflag_args() {
        let parser_state = ParserState::new();
        let mut working_set = ParserWorkingSet::new(&parser_state);

        let sig = Signature::build("foo")
            .named("--jazz", SyntaxShape::Int, "jazz!!", Some('j'))
            .named("--math", SyntaxShape::Int, "math!!", Some('m'));
        working_set.add_decl(sig.into());
        let (_, err) = working_set.parse_source(b"foo -mj", true);
        assert!(matches!(
            err,
            Some(ParseError::ShortFlagBatchCantTakeArg(..))
        ));
    }

    #[test]
    pub fn parse_call_unknown_shorthand() {
        let parser_state = ParserState::new();
        let mut working_set = ParserWorkingSet::new(&parser_state);

        let sig = Signature::build("foo").switch("--jazz", "jazz!!", Some('j'));
        working_set.add_decl(sig.into());
        let (_, err) = working_set.parse_source(b"foo -mj", true);
        assert!(matches!(err, Some(ParseError::UnknownFlag(..))));
    }

    #[test]
    pub fn parse_call_extra_positional() {
        let parser_state = ParserState::new();
        let mut working_set = ParserWorkingSet::new(&parser_state);

        let sig = Signature::build("foo").switch("--jazz", "jazz!!", Some('j'));
        working_set.add_decl(sig.into());
        let (_, err) = working_set.parse_source(b"foo -j 100", true);
        assert!(matches!(err, Some(ParseError::ExtraPositional(..))));
    }

    #[test]
    pub fn parse_call_missing_req_positional() {
        let parser_state = ParserState::new();
        let mut working_set = ParserWorkingSet::new(&parser_state);

        let sig = Signature::build("foo").required("jazz", SyntaxShape::Int, "jazz!!");
        working_set.add_decl(sig.into());
        let (_, err) = working_set.parse_source(b"foo", true);
        assert!(matches!(err, Some(ParseError::MissingPositional(..))));
    }

    #[test]
    pub fn parse_call_missing_req_flag() {
        let parser_state = ParserState::new();
        let mut working_set = ParserWorkingSet::new(&parser_state);

        let sig =
            Signature::build("foo").required_named("--jazz", SyntaxShape::Int, "jazz!!", None);
        working_set.add_decl(sig.into());
        let (_, err) = working_set.parse_source(b"foo", true);
        assert!(matches!(err, Some(ParseError::MissingRequiredFlag(..))));
    }
}
