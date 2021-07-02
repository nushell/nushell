use std::ops::{Index, IndexMut};

use crate::{
    lex, lite_parse,
    parser_state::{Type, VarId},
    DeclId, LiteBlock, ParseError, ParserWorkingSet, Span,
};

/// The syntactic shapes that values must match to be passed into a command. You can think of this as the type-checking that occurs when you call a function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyntaxShape {
    /// A specific match to a word or symbol
    Word(Vec<u8>),
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
    Call(Call),
    Garbage,
}

#[derive(Debug, Clone)]
pub struct Expression {
    expr: Expr,
    ty: Type,
    span: Span,
}
impl Expression {
    pub fn garbage(span: Span) -> Expression {
        Expression {
            expr: Expr::Garbage,
            span,
            ty: Type::Unknown,
        }
    }
}

#[derive(Debug)]
pub enum Import {}

#[derive(Debug)]
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

#[derive(Debug)]
pub struct VarDecl {
    var_id: VarId,
    expression: Expression,
}

#[derive(Debug)]
pub enum Statement {
    Pipeline(Pipeline),
    VarDecl(VarDecl),
    Import(Import),
    Expression(Expression),
    None,
}

#[derive(Debug)]
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

fn span(spans: &[Span]) -> Span {
    let length = spans.len();

    if length == 0 {
        Span::unknown()
    } else if length == 1 || spans[0].file_id != spans[length - 1].file_id {
        spans[0]
    } else {
        Span {
            start: spans[0].start,
            end: spans[length - 1].end,
            file_id: spans[0].file_id,
        }
    }
}

impl ParserWorkingSet {
    pub fn parse_external_call(&mut self, spans: &[Span]) -> (Expression, Option<ParseError>) {
        // TODO: add external parsing
        (Expression::garbage(spans[0]), None)
    }

    pub fn parse_call(&mut self, spans: &[Span]) -> (Expression, Option<ParseError>) {
        let mut error = None;

        // assume spans.len() > 0?
        let name = self.get_span_contents(spans[0]);

        if let Some(decl_id) = self.find_decl(name) {
            let mut call = Call::new();

            let sig = self
                .get_decl(decl_id)
                .expect("internal error: bad DeclId")
                .clone();

            let mut positional_idx = 0;
            let mut arg_offset = 1;

            while arg_offset < spans.len() {
                let arg_span = spans[arg_offset];
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
                                    let (arg, err) = self.parse_arg(span, arg_shape.clone());
                                    error = error.or(err);

                                    call.named.push((long_name, Some(arg)));
                                } else if let Some(arg) = spans.get(arg_offset + 1) {
                                    let (arg, err) = self.parse_arg(*arg, arg_shape.clone());
                                    error = error.or(err);

                                    call.named.push((long_name, Some(arg)));
                                    arg_offset += 1;
                                } else {
                                    error = error.or(Some(ParseError::MissingFlagParam(arg_span)))
                                }
                            }
                        } else {
                            error = error.or(Some(ParseError::UnknownFlag(arg_span)))
                        }
                    } else {
                        error = error.or(Some(ParseError::NonUtf8(arg_span)))
                    }
                } else if arg_contents.starts_with(&[b'-']) && arg_contents.len() > 1 {
                    let short_flags = &arg_contents[1..];
                    let mut found_short_flags = vec![];
                    let mut unmatched_short_flags = vec![];
                    for short_flag in short_flags.iter().enumerate() {
                        let short_flag_char = char::from(*short_flag.1);
                        let orig = arg_span;
                        let short_flag_span = Span {
                            start: orig.start + 1 + short_flag.0,
                            end: orig.start + 1 + short_flag.0 + 1,
                            file_id: orig.file_id,
                        };
                        if let Some(flag) = sig.get_short_flag(short_flag_char) {
                            // If we require an arg and are in a batch of short flags, error
                            if !found_short_flags.is_empty() && flag.arg.is_some() {
                                error = error.or(Some(ParseError::ShortFlagBatchCantTakeArg(
                                    short_flag_span,
                                )))
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
                                let (arg, err) = self.parse_arg(arg_span, positional.shape);

                                if err.is_some() {
                                    if let Some(first) = unmatched_short_flags.first() {
                                        error = error.or(Some(ParseError::UnknownFlag(*first)));
                                    }
                                } else {
                                    // We have successfully found a positional argument, move on
                                    call.positional.push(arg);
                                    positional_idx += 1;
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

                    for flag in found_short_flags {
                        if let Some(arg_shape) = flag.arg {
                            if let Some(arg) = spans.get(arg_offset + 1) {
                                let (arg, err) = self.parse_arg(*arg, arg_shape.clone());
                                error = error.or(err);

                                call.named.push((flag.long.clone(), Some(arg)));
                                arg_offset += 1;
                            } else {
                                error = error.or(Some(ParseError::MissingFlagParam(arg_span)))
                            }
                        } else {
                            call.named.push((flag.long.clone(), None));
                        }
                    }
                } else if let Some(positional) = sig.get_positional(positional_idx) {
                    let (arg, err) = self.parse_arg(arg_span, positional.shape);
                    error = error.or(err);

                    call.positional.push(arg);
                } else {
                    error = error.or(Some(ParseError::ExtraPositional(arg_span)))
                }
                arg_offset += 1;
            }

            // FIXME: type unknown
            (
                Expression {
                    expr: Expr::Call(call),
                    ty: Type::Unknown,
                    span: span(spans),
                },
                error,
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
                        ty: Type::Int,
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
                        ty: Type::Int,
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
                        ty: Type::Int,
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
                    ty: Type::Int,
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

    pub fn parse_arg(
        &mut self,
        span: Span,
        shape: SyntaxShape,
    ) -> (Expression, Option<ParseError>) {
        let bytes = self.get_span_contents(span);
        if !bytes.is_empty() && bytes[0] == b'$' {
            if let Some(var_id) = self.find_variable(bytes) {
                let ty = *self
                    .get_variable(var_id)
                    .expect("internal error: invalid VarId");
                return (
                    Expression {
                        expr: Expr::Var(var_id),
                        ty,
                        span,
                    },
                    None,
                );
            } else {
                return (garbage(span), Some(ParseError::VariableNotFound(span)));
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
                        Some(ParseError::Mismatch("number".into(), span)),
                    )
                }
            }
            _ => (
                garbage(span),
                Some(ParseError::Mismatch("number".into(), span)),
            ),
        }
    }

    pub fn parse_math_expression(&mut self, spans: &[Span]) -> (Expression, Option<ParseError>) {
        self.parse_arg(spans[0], SyntaxShape::Number)
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
        let mut error = None;
        if spans.len() >= 4 && self.parse_keyword(spans[0], b"let").is_none() {
            let (_, err) = self.parse_variable(spans[1]);
            error = error.or(err);

            let err = self.parse_keyword(spans[2], b"=");
            error = error.or(err);

            let (expression, err) = self.parse_expression(&spans[3..]);
            error = error.or(err);

            let var_name: Vec<_> = self.get_span_contents(spans[1]).into();
            let var_id = self.add_variable(var_name, expression.ty);

            (Statement::VarDecl(VarDecl { var_id, expression }), error)
        } else {
            let span = span(spans);
            (
                Statement::Expression(garbage(span)),
                Some(ParseError::Mismatch("let".into(), span)),
            )
        }
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

    pub fn parse_file(&mut self, fname: &str, contents: &[u8]) -> (Block, Option<ParseError>) {
        let mut error = None;

        let file_id = self.add_file(fname.into(), contents.into());

        let (output, err) = lex(contents, file_id, 0, crate::LexMode::Normal);
        error = error.or(err);

        let (output, err) = lite_parse(&output);
        error = error.or(err);

        let (output, err) = self.parse_block(&output);
        error = error.or(err);

        (output, error)
    }

    pub fn parse_source(&mut self, source: &[u8]) -> (Block, Option<ParseError>) {
        let mut error = None;

        let file_id = self.add_file("source".into(), source.into());

        let (output, err) = lex(source, file_id, 0, crate::LexMode::Normal);
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
    use crate::Signature;

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
        assert!(matches!(
            block[0],
            Statement::Expression(Expression {
                expr: Expr::Call(Call { decl_id: 0, .. }),
                ..
            })
        ));
    }
}
