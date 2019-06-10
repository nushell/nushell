use crate::errors::ShellError;
use derive_new::new;
use log::trace;
use logos_derive::Logos;
use std::collections::VecDeque;
use std::ops::Range;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Logos)]
#[extras = "LexerState"]
crate enum TopToken {
    #[error]
    Error,

    #[end]
    END,

    #[token = "function"]
    #[callback = "after_function"]
    Function,

    #[regex = "-?[0-9]+"]
    #[callback = "after_num"]
    Num,

    #[regex = r#"'([^']|\\')*'"#]
    SQString,

    #[regex = r#""([^"]|\\")*""#]
    DQString,

    #[token = "$"]
    #[callback = "start_variable"]
    Dollar,

    #[regex = r#"[^\s0-9"'$\-(){}][^\s"'(){}]*"#]
    #[callback = "end_bare_variable"]
    Bare,

    #[token = "|"]
    Pipe,

    #[token = "."]
    Dot,

    #[token = "{"]
    OpenBrace,

    #[token = "}"]
    CloseBrace,

    #[token = "("]
    OpenParen,

    #[token = ")"]
    CloseParen,

    #[token = ">"]
    OpGt,

    #[token = "<"]
    OpLt,

    #[token = ">="]
    OpGte,

    #[token = "<="]
    OpLte,

    #[token = "=="]
    OpEq,

    #[token = "!="]
    OpNeq,

    #[token = "--"]
    DashDash,

    #[token = "-"]
    Dash,

    #[regex = r"[^\S\r\n]"]
    Whitespace,

    #[regex = r"(\r\n|\n)"]
    Newline,
}

impl TopToken {
    fn to_token(&self, _text: &str) -> Option<Token> {
        use TopToken::*;

        let result = match self {
            END => return None,
            Function => Token::KeywordFunction,
            Newline => Token::Newline,
            Num => Token::Num,
            SQString => Token::SQString,
            DQString => Token::DQString,
            Dollar => Token::Dollar,
            Bare => Token::Bare,
            Pipe => Token::Pipe,
            Dot => Token::Bare,
            OpenBrace => Token::OpenBrace,
            CloseBrace => Token::CloseBrace,
            OpenParen => Token::OpenParen,
            CloseParen => Token::CloseParen,
            OpGt => Token::OpGt,
            OpLt => Token::OpLt,
            OpGte => Token::OpGte,
            OpLte => Token::OpLte,
            OpEq => Token::OpEq,
            OpNeq => Token::OpNeq,
            DashDash => Token::DashDash,
            Dash => Token::Dash,
            Whitespace => Token::Whitespace,
            Error => unreachable!("Don't call to_token with the error variant"),
        };

        Some(result)
    }
}

fn after_num<S>(lex: &mut logos::Lexer<TopToken, S>) {
    trace!("after_num EXTRAS={:?}", lex.extras);
    lex.extras.push(LexerStateName::AfterNum);
}

fn after_function<S>(lex: &mut logos::Lexer<TopToken, S>) {
    trace!("after_function EXTRAS={:?}", lex.extras);
    lex.extras.push(LexerStateName::AfterFunction);
}

fn start_variable<S>(lex: &mut logos::Lexer<TopToken, S>) {
    trace!("start_variable EXTRAS={:?}", lex.extras);
    lex.extras.push(LexerStateName::VariableToken);
}

fn end_bare_variable<S>(lex: &mut logos::Lexer<TopToken, S>) {
    trace!("end_bare_variable EXTRAS={:?}", lex.extras);
    lex.extras.push(LexerStateName::AfterVariableToken);
}

#[derive(Logos, Debug, Clone, Copy, Eq, PartialEq)]
#[extras = "LexerState"]
crate enum AfterFunction {
    #[error]
    Error,

    #[end]
    END,

    #[regex = r"[A-Za-z][A-Za-z0-9_\-]*"]
    #[callback = "after_function_name"]
    CommandName,

    #[token = "{"]
    #[callback = "start_function_block"]
    OpenBrace,

    #[regex = r"[^\S\r\n]"]
    Whitespace,
}

impl AfterFunction {
    fn to_token(&self) -> Option<Token> {
        use AfterFunction::*;

        let result = match self {
            END => return None,
            CommandName => Token::CommandName,
            Whitespace => Token::Whitespace,
            OpenBrace => Token::OpenBrace,
            Error => unreachable!("Don't call to_token with the error variant"),
        };

        Some(result)
    }
}

fn after_function_name<S>(lex: &mut logos::Lexer<AfterFunction, S>) {
    trace!("after_function_name EXTRAS={:?}", lex.extras);
    lex.extras.push(LexerStateName::AfterFunctionName);
}

fn start_function_block<S>(lex: &mut logos::Lexer<AfterFunction, S>) {
    trace!("start_function_block EXTRAS={:?}", lex.extras);
    lex.extras.pop();
}

#[derive(Logos, Debug, Clone, Copy, Eq, PartialEq)]
#[extras = "LexerState"]
crate enum AfterFunctionName {
    #[error]
    Error,

    #[end]
    END,

    #[token = "("]
    #[callback = "start_param_list"]
    StartParamList,

    #[regex = r"[^\S\r\n]"]
    Whitespace,
}

impl AfterFunctionName {
    fn to_token(&self) -> Option<Token> {
        use AfterFunctionName::*;

        let result = match self {
            END => return None,
            StartParamList => Token::StartParamList,
            Whitespace => Token::Whitespace,
            Error => unreachable!("Don't call to_token with the error variant"),
        };

        Some(result)
    }
}

fn start_param_list<S>(lex: &mut logos::Lexer<AfterFunctionName, S>) {
    trace!("start_param_list EXTRAS={:?}", lex.extras);
    lex.extras.push(LexerStateName::InParamList);
}

#[derive(Logos, Debug, Clone, Copy, Eq, PartialEq)]
#[extras = "LexerState"]
crate enum InParamList {
    #[error]
    Error,

    #[end]
    END,

    #[token = "$"]
    #[callback = "start_param_name"]
    Dollar,

    #[regex = r"[^\S\r\n]"]
    Whitespace,
}

impl InParamList {
    fn to_token(&self) -> Option<Token> {
        use InParamList::*;

        let result = match self {
            END => return None,
            Dollar => Token::Dollar,
            Whitespace => Token::Whitespace,
            Error => unreachable!("Don't call to_token with the error variant"),
        };

        Some(result)
    }
}

fn start_param_name<S>(lex: &mut logos::Lexer<InParamList, S>) {
    trace!("start_param_name EXTRAS={:?}", lex.extras);
    lex.extras.push(LexerStateName::VariableToken);
}

#[derive(Logos, Debug, Clone, Copy, Eq, PartialEq)]
#[extras = "LexerState"]
crate enum TypeTokens {
    #[error]
    Error,

    #[end]
    END,

    #[regex = "(any|int|decimal|bytes|text|boolean|date|object|list|block)"]
    #[callback = "end_type_token"]
    TypeName,
}

fn end_type_token<S>(lex: &mut logos::Lexer<TypeTokens, S>) {
    trace!("end_type_token EXTRAS={:?}", lex.extras);
    lex.extras.pop();
}

impl TypeTokens {
    fn to_token(&self, text: &str) -> Option<Token> {
        use TypeTokens::*;

        let result = match self {
            END => return None,
            TypeName => match text {
                "any" => Token::TyAny,
                "int" => Token::TyInt,
                "decimal" => Token::TyDecimal,
                "bytes" => Token::TyBytes,
                "text" => Token::TyText,
                "boolean" => Token::TyBoolean,
                "date" => Token::TyDate,
                "object" => Token::TyObject,
                "list" => Token::TyList,
                other => unreachable!("Type name {:?} shouldn't be possible", other),
            },
            Error => unreachable!("Don't call to_token with the error variant"),
        };

        Some(result)
    }
}

#[derive(Logos, Debug, Clone, Copy, Eq, PartialEq)]
#[extras = "LexerState"]
crate enum AfterNum {
    #[error]
    Error,

    #[end]
    END,

    #[regex = "(B|KB|MB|GB|TB|PB)"]
    #[callback = "end_unit"]
    Unit,

    #[regex = r"[^\S\r\n]"]
    #[callback = "end_number"]
    Whitespace,

    #[regex = r"(\r\n|\n)"]
    Newline,
}

impl AfterNum {
    fn to_token(&self) -> Option<Token> {
        use AfterNum::*;

        let result = match self {
            END => return None,
            Unit => Token::Unit,
            Whitespace => Token::Whitespace,
            Newline => Token::Newline,
            Error => unreachable!("Don't call to_token with the error variant"),
        };

        Some(result)
    }
}

fn end_unit<S>(lex: &mut logos::Lexer<AfterNum, S>) {
    trace!("end_unit EXTRAS={:?}", lex.extras);
    lex.extras.pop();
}

fn end_number<S>(lex: &mut logos::Lexer<AfterNum, S>) {
    trace!("end_number EXTRAS={:?}", lex.extras);
    lex.extras.pop();
}

#[derive(Logos, Debug, Clone, Copy, Eq, PartialEq)]
#[extras = "LexerState"]
crate enum VariableToken {
    #[error]
    Error,

    #[end]
    END,

    #[regex = r"[A-Za-z][A-Za-z0-9\-?!]*"]
    #[callback = "end_variable"]
    Variable,
}

impl VariableToken {
    fn to_token(&self, _text: &str) -> Option<Token> {
        use VariableToken::*;

        let result = match self {
            END => return None,
            Variable => Token::Variable,
            Error => unreachable!("Don't call to_token with the error variant"),
        };

        Some(result)
    }
}

fn end_variable<S>(lex: &mut logos::Lexer<VariableToken, S>) {
    trace!("end_variable EXTRAS={:?}", lex.extras);
    lex.extras.replace(LexerStateName::AfterVariableToken);
}

#[derive(Logos, Debug, Clone, Copy, Eq, PartialEq)]
#[extras = "LexerState"]
crate enum AfterVariableToken {
    #[error]
    Error,

    #[end]
    END,

    #[token = "."]
    #[callback = "start_member"]
    Dot,

    #[token = ":"]
    #[callback = "start_type"]
    Colon,

    #[regex = r"[^\S\r\n]"]
    #[callback = "terminate_variable"]
    Whitespace,

    #[token = ")"]
    #[callback = "end_param_list"]
    EndParamList,

    #[regex = r"(\r\n|\n)"]
    Newline,
}

impl AfterVariableToken {
    fn to_token(&self) -> Option<Token> {
        use AfterVariableToken::*;

        let result = match self {
            END => return None,
            Dot => Token::PathDot,
            Colon => Token::Colon,
            Whitespace => Token::Whitespace,
            Newline => Token::Newline,
            EndParamList => Token::EndParamList,
            Error => unreachable!("Don't call to_token with the error variant"),
        };

        Some(result)
    }
}

fn start_member<S>(lex: &mut logos::Lexer<AfterVariableToken, S>) {
    trace!("start_member EXTRAS={:?}", lex.extras);
    lex.extras.push(LexerStateName::AfterMemberDot);
}

fn end_param_list<S>(lex: &mut logos::Lexer<AfterVariableToken, S>) {
    trace!("end_param_list EXTRAS={:?}", lex.extras);
    lex.extras.pop();
    lex.extras.pop();
    lex.extras.pop();
    lex.extras.push(LexerStateName::AfterParamList);
}

fn start_type<S>(lex: &mut logos::Lexer<AfterVariableToken, S>) {
    trace!("start_type EXTRAS={:?}", lex.extras);
    lex.extras.push(LexerStateName::TypeTokens);
}

fn terminate_variable<S>(lex: &mut logos::Lexer<AfterVariableToken, S>) {
    trace!("terminate_variable EXTRAS={:?}", lex.extras);
    lex.extras.pop();
}

#[derive(Logos, Debug, Clone, Copy, Eq, PartialEq)]
#[extras = "LexerState"]
crate enum AfterMemberDot {
    #[error]
    Error,

    #[end]
    END,

    #[regex = r"[A-Za-z][A-Za-z0-9\-?!]*"]
    #[callback = "finish_member"]
    Member,

    #[regex = r#"'([^']|\\')*'"#]
    SQString,

    #[regex = r#""([^"]|\\")*""#]
    DQString,

    #[regex = r"[^\S\r\n]"]
    Whitespace,

    #[regex = r"(\r\n|\n)"]
    Newline,
}

impl AfterMemberDot {
    fn to_token(&self) -> Option<Token> {
        use AfterMemberDot::*;

        let result = match self {
            END => return None,
            Member => Token::Member,
            SQString => Token::SQMember,
            DQString => Token::DQMember,

            Whitespace => Token::Whitespace,
            Newline => Token::Newline,
            Error => unreachable!("Don't call to_token with the error variant"),
        };

        Some(result)
    }
}

fn finish_member<S>(lex: &mut logos::Lexer<AfterMemberDot, S>) {
    trace!("finish_member EXTRAS={:?}", lex.extras);
    lex.extras.pop();
}

#[derive(Logos, Debug, Clone, Copy, Eq, PartialEq)]
#[extras = "LexerState"]
crate enum AfterParamList {
    #[error]
    Error,

    #[token = "->"]
    #[callback = "finish_arrow"]
    Arrow,

    #[end]
    END,
}

impl AfterParamList {
    fn to_token(&self, _text: &str) -> Option<Token> {
        use AfterParamList::*;

        let result = match self {
            END => return None,
            Arrow => Token::ReturnArrow,
            Error => unreachable!("Don't call to_token with the error variant"),
        };

        Some(result)
    }
}

fn finish_arrow<S>(lex: &mut logos::Lexer<AfterParamList, S>) {
    trace!("finish_arrow EXTRAS={:?}", lex.extras);
    lex.extras.replace(LexerStateName::TypeTokens);
}

#[derive(Debug, Clone, Copy)]
crate enum LexerStateName {
    Top,
    VariableToken,
    AfterFunction,
    AfterFunctionName,
    TypeTokens,
    InParamList,
    AfterParamList,
    AfterMemberDot,
    AfterNum,
    AfterVariableToken,
}

impl Default for LexerStateName {
    fn default() -> LexerStateName {
        LexerStateName::Top
    }
}

#[derive(Debug, Clone)]
crate struct LexerState {
    stack: VecDeque<LexerStateName>,
}

impl Default for LexerState {
    fn default() -> LexerState {
        let mut stack = VecDeque::new();
        stack.push_back(LexerStateName::Top);
        LexerState { stack }
    }
}

impl LexerState {
    crate fn debug_states(&self) -> String {
        let items: Vec<String> = self.stack.iter().map(|s| format!("{:?}", s)).collect();
        let debug = itertools::join(items, " -> ");
        format!("{}", debug)
    }

    crate fn push(&mut self, name: LexerStateName) {
        self.stack.push_back(name);
        trace!("push {:?} (to {})", name, self.debug_states());
    }

    crate fn replace(&mut self, name: LexerStateName) {
        self.stack.pop_back();
        self.stack.push_back(name);
        trace!("replace with {:?} (to {})", name, self.debug_states());
    }

    crate fn pop(&mut self) {
        self.stack.pop_back();
        trace!("pop (to {})", self.debug_states());
    }

    crate fn current(&self) -> LexerStateName {
        *self
            .stack
            .back()
            .expect("There must always be at least one state in the lexer stack")
    }
}

impl logos::Extras for LexerState {
    fn on_advance(&mut self) {}
    fn on_whitespace(&mut self, _byte: u8) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct Span {
    crate start: usize,
    crate end: usize,
    // source: &'source str,
}

impl From<(usize, usize)> for Span {
    fn from(input: (usize, usize)) -> Span {
        Span {
            start: input.0,
            end: input.1,
        }
    }
}

impl From<&std::ops::Range<usize>> for Span {
    fn from(input: &std::ops::Range<usize>) -> Span {
        Span {
            start: input.start,
            end: input.end,
        }
    }
}

impl Span {
    fn new(range: &Range<usize>) -> Span {
        Span {
            start: range.start,
            end: range.end,
            // source,
        }
    }
}

impl language_reporting::ReportingSpan for Span {
    fn with_start(&self, start: usize) -> Self {
        Span {
            start,
            end: self.end,
        }
    }

    fn with_end(&self, end: usize) -> Self {
        Span {
            start: self.start,
            end,
        }
    }

    fn start(&self) -> usize {
        self.start
    }

    fn end(&self) -> usize {
        self.end
    }
}

#[derive(new, Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Spanned<T> {
    crate span: Span,
    crate item: T,
}

impl<T> std::ops::Deref for Spanned<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.item
    }
}

impl<T> Spanned<T> {
    crate fn from_item(item: T, span: impl Into<Span>) -> Spanned<T> {
        Spanned {
            span: span.into(),
            item,
        }
    }

    crate fn map<U>(self, input: impl FnOnce(T) -> U) -> Spanned<U> {
        let Spanned { span, item } = self;

        let mapped = input(item);
        Spanned { span, item: mapped }
    }
}

#[derive(new, Debug, Clone, Eq, PartialEq)]
pub struct SpannedToken<'source> {
    crate span: Span,
    crate slice: &'source str,
    crate token: Token,
}

impl SpannedToken<'source> {
    crate fn to_spanned_string(&self) -> Spanned<String> {
        Spanned::from_item(self.slice.to_string(), self.span)
    }

    crate fn as_slice(&self) -> &str {
        self.slice
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Token {
    KeywordFunction,
    CommandName,
    #[allow(unused)]
    Comma,
    Colon,
    Variable,
    PathDot,
    ReturnArrow,
    Member,
    SQMember,
    DQMember,
    Num,
    SQString,
    DQString,
    Unit,
    Dollar,
    Bare,
    Pipe,
    OpenBrace,
    CloseBrace,
    OpenParen,
    CloseParen,
    StartParamList,
    EndParamList,
    OpGt,
    OpLt,
    OpGte,
    OpLte,
    OpEq,
    OpNeq,
    Dash,
    DashDash,
    TyAny,
    #[allow(unused)]
    TyBlock,
    TyBoolean,
    TyBytes,
    TyDate,
    TyDecimal,
    TyInt,
    TyList,
    TyObject,
    TyText,
    Whitespace,
    Newline,
}

#[derive(Clone)]
crate struct Lexer<'source> {
    lexer: logos::Lexer<TopToken, &'source str>,
    first: bool,
    whitespace: bool,
}

impl Lexer<'source> {
    crate fn new(source: &str, whitespace: bool) -> Lexer<'_> {
        Lexer {
            first: true,
            lexer: logos::Logos::lexer(source),
            whitespace,
        }
    }

    fn lex_error(&self, range: &Range<usize>) -> ShellError {
        use language_reporting::*;

        let states = self.lexer.extras.debug_states();

        ShellError::diagnostic(
            Diagnostic::new(Severity::Error, "Lex error")
                .with_label(Label::new_primary(Span::new(range)).with_message(states)),
        )
    }
}

impl Iterator for Lexer<'source> {
    type Item = Result<(usize, SpannedToken<'source>, usize), ShellError>;

    fn next(&mut self) -> Option<Self::Item> {
        let result = if self.first {
            self.first = false;

            match self.lexer.token {
                TopToken::Error => Some(Err(self.lex_error(&self.lexer.range()))),
                TopToken::Whitespace if !self.whitespace => self.next(),
                other => spanned(
                    other.to_token(self.lexer.slice())?,
                    self.lexer.slice(),
                    &self.lexer.range(),
                ),
            }
        } else {
            trace!("STATE={:?}", self.lexer.extras);

            match self.lexer.extras.current() {
                LexerStateName::Top => {
                    let (lexer, range, slice, token) = advance::<TopToken>(self.lexer.clone());
                    self.lexer = lexer;

                    match token {
                        TopToken::Error => Some(Err(self.lex_error(&range))),
                        TopToken::Whitespace if !self.whitespace => self.next(),
                        other => spanned(other.to_token(slice)?, slice, &range),
                    }
                }

                LexerStateName::AfterParamList => {
                    let (lexer, range, slice, token) =
                        advance::<AfterParamList>(self.lexer.clone());
                    self.lexer = lexer;

                    match token {
                        AfterParamList::Error => Some(Err(self.lex_error(&range))),
                        other => spanned(other.to_token(slice)?, slice, &range),
                    }
                }

                LexerStateName::TypeTokens => {
                    let (lexer, range, slice, token) = advance::<TypeTokens>(self.lexer.clone());
                    self.lexer = lexer;

                    match token {
                        TypeTokens::Error => Some(Err(self.lex_error(&range))),
                        other => spanned(other.to_token(slice)?, slice, &range),
                    }
                }

                LexerStateName::AfterNum => {
                    let (lexer, range, slice, token) = advance::<AfterNum>(self.lexer.clone());
                    self.lexer = lexer;

                    match token {
                        AfterNum::Error => Some(Err(self.lex_error(&range))),
                        AfterNum::Whitespace if !self.whitespace => self.next(),
                        other => spanned(other.to_token()?, slice, &range),
                    }
                }

                LexerStateName::AfterFunction => {
                    let (lexer, range, slice, token) = advance::<AfterFunction>(self.lexer.clone());
                    self.lexer = lexer;

                    match token {
                        AfterFunction::Error => Some(Err(self.lex_error(&range))),
                        AfterFunction::Whitespace if !self.whitespace => self.next(),
                        other => spanned(other.to_token()?, slice, &range),
                    }
                }

                LexerStateName::AfterFunctionName => {
                    let (lexer, range, slice, token) =
                        advance::<AfterFunctionName>(self.lexer.clone());
                    self.lexer = lexer;

                    match token {
                        AfterFunctionName::Error => Some(Err(self.lex_error(&range))),
                        AfterFunctionName::Whitespace if !self.whitespace => self.next(),
                        other => spanned(other.to_token()?, slice, &range),
                    }
                }

                LexerStateName::InParamList => {
                    let (lexer, range, slice, token) = advance::<InParamList>(self.lexer.clone());
                    self.lexer = lexer;

                    match token {
                        InParamList::Error => Some(Err(self.lex_error(&range))),
                        InParamList::Whitespace if !self.whitespace => self.next(),
                        other => return spanned(other.to_token()?, slice, &range),
                    }
                }

                LexerStateName::AfterMemberDot => {
                    let (lexer, range, slice, token) =
                        advance::<AfterMemberDot>(self.lexer.clone());
                    self.lexer = lexer;

                    match token {
                        AfterMemberDot::Error => return Some(Err(self.lex_error(&range))),
                        AfterMemberDot::Whitespace if !self.whitespace => self.next(),
                        other => spanned(other.to_token()?, slice, &range),
                    }
                }

                LexerStateName::AfterVariableToken => {
                    let (lexer, range, slice, token) =
                        advance::<AfterVariableToken>(self.lexer.clone());
                    self.lexer = lexer;

                    match token {
                        AfterVariableToken::Error => Some(Err(self.lex_error(&range))),
                        AfterVariableToken::Whitespace if !self.whitespace => self.next(),
                        other => spanned(other.to_token()?, slice, &range),
                    }
                }

                LexerStateName::VariableToken => {
                    let (lexer, range, slice, token) = advance::<VariableToken>(self.lexer.clone());
                    self.lexer = lexer;

                    match token {
                        VariableToken::Error => Some(Err(self.lex_error(&range))),
                        other => spanned(other.to_token(slice)?, slice, &range),
                    }
                }
            }
        };

        trace!("emitting {:?}", result);

        result
    }
}

fn spanned<'source>(
    token: Token,
    slice: &'source str,
    range: &Range<usize>,
) -> Option<Result<(usize, SpannedToken<'source>, usize), ShellError>> {
    let token = SpannedToken::new(Span::new(range), slice, token);
    Some(Ok((range.start, token, range.end)))
}

fn advance<T>(
    lexer: logos::Lexer<TopToken, &'source str>,
) -> (
    logos::Lexer<TopToken, &'source str>,
    Range<usize>,
    &'source str,
    T,
)
where
    T: logos::Logos<Extras = LexerState> + logos::source::WithSource<&'source str> + Copy,
{
    let lexer = lexer.advance_as::<T>();
    let token = &lexer.token;
    let range = lexer.range();
    let slice = lexer.slice();
    (lexer.clone().morph::<TopToken>(), range, slice, *token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn assert_lex(source: &str, tokens: &[TestToken<'_>]) {
        let _ = pretty_env_logger::try_init();

        let lex = Lexer::new(source, false);
        let mut current = 0;

        let expected_tokens: Vec<SpannedToken> = tokens
            .iter()
            .filter_map(|token_desc| {
                trace!("{:?}", token_desc);

                let len = token_desc.source.len();
                let range = current..(current + len);
                let token = token_desc.to_token(&range);

                current = current + len;

                if let SpannedToken {
                    token: Token::Whitespace,
                    ..
                } = token
                {
                    None
                } else {
                    Some(token)
                }
            })
            .collect();

        let actual_tokens: Result<Vec<SpannedToken>, _> =
            lex.map(|result| result.map(|(_, i, _)| i)).collect();

        let actual_tokens = match actual_tokens {
            Ok(v) => v,
            Err(ShellError::Diagnostic(diag)) => {
                use language_reporting::termcolor;

                let writer = termcolor::StandardStream::stdout(termcolor::ColorChoice::Auto);
                let files = crate::parser::span::Files::new(source.to_string());

                language_reporting::emit(
                    &mut writer.lock(),
                    &files,
                    &diag.diagnostic,
                    &language_reporting::DefaultConfig,
                )
                .unwrap();

                panic!("Test failed")
            }
            Err(err) => panic!("Something went wrong during lex: {:#?}", err),
        };

        assert_eq!(actual_tokens, expected_tokens);
    }

    #[derive(Debug)]
    enum TokenDesc {
        Ws,
        Member,
        PathDot,
        Top(TopToken),
        Var(VariableToken),
    }

    #[derive(Debug, new)]
    struct TestToken<'source> {
        desc: TokenDesc,
        source: &'source str,
    }

    impl TestToken<'source> {
        fn to_token(&self, range: &std::ops::Range<usize>) -> SpannedToken<'source> {
            match self.desc {
                TokenDesc::Top(tok) => SpannedToken::new(
                    Span::new(range),
                    self.source,
                    tok.to_token(&self.source).unwrap(),
                ),
                TokenDesc::Var(tok) => SpannedToken::new(
                    Span::new(range),
                    self.source,
                    tok.to_token(&self.source).unwrap(),
                ),
                TokenDesc::Member => {
                    SpannedToken::new(Span::new(range), self.source, Token::Member)
                }

                TokenDesc::Ws => {
                    SpannedToken::new(Span::new(range), self.source, Token::Whitespace)
                }

                TokenDesc::PathDot => {
                    SpannedToken::new(Span::new(range), self.source, Token::PathDot)
                }
            }
        }
    }

    macro_rules! chomp_tokens {
        { rest = { SP $($rest:tt)* }, accum = [ $($accum:tt)* ] } => {
            chomp_tokens! { rest = { $($rest)* }, accum = [ $($accum)* { SP } ] }
        };

        { rest = { ws($expr:expr) $($rest:tt)* }, accum = [ $($accum:tt)* ] } => {
            chomp_tokens! { rest = { $($rest)* }, accum = [ $($accum)* { ws($expr) } ] }
        };

        { rest = { $id:ident ( $expr:expr ) $($rest:tt)* }, accum = [ $($accum:tt)* ] } => {
            chomp_tokens! { rest = { $($rest)* }, accum = [ $($accum)* { tok(stringify!($id), $expr) } ] }
        };

        { rest = { $token:tt $($rest:tt)* }, accum = [ $($accum:tt)* ] } => {
            chomp_tokens! { rest = { $($rest)* }, accum = [ $($accum)* { tk($token) } ] }
        };

        { rest = { }, accum = [ $({ $($tokens:tt)* })* ] } => {
            &[ $($($tokens)*),* ]
        }
    }

    macro_rules! tokens {
        ($($tokens:tt)*) => {
            chomp_tokens! { rest = { $($tokens)* }, accum = [] }
        };
    }

    #[test]
    fn test_tokenize_number() {
        assert_lex("123", tokens![Num("123")]);
        // assert_lex("123", &[tok("Num", "123")]);
        assert_lex(
            "123 456 789",
            tokens![Num("123") SP Num("456") SP Num("789")],
        );

        assert_lex("-123", tokens![Num("-123")]);

        assert_lex(
            "123   -456    789",
            tokens![
                Num("123")
                ws("   ")
                Num("-456")
                ws("    ")
                Num("789")
            ],
        )
    }

    #[test]
    fn test_tokenize_variable() {
        assert_lex("$var", tokens![ "$" Var("var")]);
    }

    #[test]
    fn test_tokenize_string() {
        assert_lex(
            r#" "hello world" "#,
            tokens![ SP DQString(r#""hello world""#) SP ],
        );

        assert_lex(
            r#" 'hello world' "#,
            tokens![ SP SQString(r#"'hello world'"#) SP ],
        );
    }

    #[test]
    fn test_tokenize_path() {
        assert_lex("$var.bar", tokens![ "$" Var("var") "???." Member("bar") ]);
        assert_lex("$it.bar", tokens![ "$" Var("it") "???." Member("bar") ]);
        assert_lex(
            "$var. bar",
            tokens![ "$" Var("var") "???." SP Member("bar") ],
        );
        assert_lex("$it. bar", tokens![ "$" Var("it") "???." SP Member("bar") ]);
    }

    #[test]
    fn test_tokenize_operator() {
        assert_lex(
            "$it.cpu > 10",
            tokens![ "$" Var("it") "???." Member("cpu") SP ">" SP Num("10") ],
        );

        assert_lex(
            "$it.cpu < 10",
            tokens![ "$" Var("it") "???." Member("cpu") SP "<" SP Num("10") ],
        );

        assert_lex(
            "$it.cpu >= 10",
            tokens![ "$" Var("it") "???." Member("cpu") SP ">=" SP Num("10") ],
        );

        assert_lex(
            "$it.cpu <= 10",
            tokens![ "$" Var("it") "???." Member("cpu") SP "<=" SP Num("10") ],
        );

        assert_lex(
            "$it.cpu == 10",
            tokens![ "$" Var("it") "???." Member("cpu") SP "==" SP Num("10") ],
        );

        assert_lex(
            "$it.cpu != 10",
            tokens![ "$" Var("it") "???." Member("cpu") SP "!=" SP Num("10") ],
        );
    }

    #[test]
    fn test_tokenize_smoke() {
        assert_lex(
            "ls | where cpu > 10",
            tokens![ Bare("ls") SP "|" SP Bare("where") SP Bare("cpu") SP ">" SP Num("10") ],
        );

        assert_lex(
            "ls | where { $it.cpu > 10 }",
            tokens![ Bare("ls") SP "|" SP Bare("where") SP "{" SP "$" Var("it") "???." Member("cpu") SP ">" SP Num("10") SP "}" ],
        );

        assert_lex(
            "open input2.json | from-json | select glossary",
            tokens![ Bare("open") SP Bare("input2.json") SP "|" SP Bare("from-json") SP "|" SP Bare("select") SP Bare("glossary") ],
        );

        assert_lex(
            "git add . -v",
            tokens![ Bare("git") SP Bare("add") SP Bare(".") SP "-" Bare("v") ],
        )
    }

    fn tok(name: &str, value: &'source str) -> TestToken<'source> {
        match name {
            "Num" => TestToken::new(TokenDesc::Top(TopToken::Num), value),
            "Var" => TestToken::new(TokenDesc::Var(VariableToken::Variable), value),
            "Member" => TestToken::new(TokenDesc::Member, value),
            "Bare" => TestToken::new(TokenDesc::Top(TopToken::Bare), value),
            "DQString" => TestToken::new(TokenDesc::Top(TopToken::DQString), value),
            "SQString" => TestToken::new(TokenDesc::Top(TopToken::SQString), value),
            other => panic!("Unexpected token name in test: {}", other),
        }
    }

    fn tk(name: &'source str) -> TestToken<'source> {
        let token = match name {
            "???." => return TestToken::new(TokenDesc::PathDot, "."),
            "." => TopToken::Dot,
            "--" => TopToken::DashDash,
            "-" => TopToken::Dash,
            "$" => TopToken::Dollar,
            "|" => TopToken::Pipe,
            "{" => TopToken::OpenBrace,
            "}" => TopToken::CloseBrace,
            ">" => TopToken::OpGt,
            "<" => TopToken::OpLt,
            ">=" => TopToken::OpGte,
            "<=" => TopToken::OpLte,
            "==" => TopToken::OpEq,
            "!=" => TopToken::OpNeq,
            other => panic!("Unexpected token name in test: {}", other),
        };

        TestToken::new(TokenDesc::Top(token), name)
    }

    const SP: TestToken<'static> = TestToken {
        desc: TokenDesc::Ws,
        source: " ",
    };

    fn ws(string: &'static str) -> TestToken<'source> {
        TestToken::new(TokenDesc::Ws, string)
    }

}
