use crate::errors::ShellError;
use derive_new::new;
use log::debug;
use logos_derive::Logos;
use std::ops::Range;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Logos)]
#[extras = "LexerState"]
crate enum TopToken {
    #[error]
    Error,

    #[end]
    END,

    #[regex = "-?[0-9]+"]
    Num,

    #[regex = r#"'([^']|\\')*'"#]
    SQString,

    #[regex = r#""([^"]|\\")*""#]
    DQString,

    #[regex = "-?[0-9]+[A-Za-z]+"]
    UnitsNum,

    #[regex = r"\$"]
    #[callback = "start_variable"]
    Dollar,

    #[regex = r#"[^\s0-9"'$\-][^\s"'\.]*"#]
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

    #[regex = r"\s+"]
    Whitespace,
}

impl TopToken {
    fn to_token(&self) -> Option<Token> {
        use TopToken::*;

        let result = match self {
            END => return None,
            Num => Token::Num,
            SQString => Token::SQString,
            DQString => Token::DQString,
            UnitsNum => Token::UnitsNum,
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

fn start_variable<S>(lex: &mut logos::Lexer<TopToken, S>) {
    debug!("start_variable EXTRAS={:?}", lex.extras);
    lex.extras.current = LexerStateName::Var;
}

fn end_bare_variable<S>(lex: &mut logos::Lexer<TopToken, S>) {
    debug!("end_variable EXTRAS={:?}", lex.extras);
    lex.extras.current = LexerStateName::AfterVariableToken;
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
    fn to_token(&self) -> Option<Token> {
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
    debug!("end_variable EXTRAS={:?}", lex.extras);
    lex.extras.current = LexerStateName::AfterVariableToken;
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

    #[regex = r"\s"]
    #[callback = "terminate_variable"]
    Whitespace,
}

impl AfterVariableToken {
    fn to_token(&self) -> Option<Token> {
        use AfterVariableToken::*;

        let result = match self {
            END => return None,
            Dot => Token::PathDot,
            Whitespace => Token::Whitespace,
            Error => unreachable!("Don't call to_token with the error variant"),
        };

        Some(result)
    }
}

fn start_member<S>(lex: &mut logos::Lexer<AfterVariableToken, S>) {
    debug!("start_variable EXTRAS={:?}", lex.extras);
    lex.extras.current = LexerStateName::AfterMemberDot;
}

fn terminate_variable<S>(lex: &mut logos::Lexer<AfterVariableToken, S>) {
    debug!("terminate_variable EXTRAS={:?}", lex.extras);
    lex.extras.current = LexerStateName::Top;
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

    #[regex = r"\s"]
    Whitespace,
}

impl AfterMemberDot {
    fn to_token(&self) -> Option<Token> {
        use AfterMemberDot::*;

        let result = match self {
            END => return None,
            Member => Token::Member,
            Whitespace => Token::Whitespace,
            Error => unreachable!("Don't call to_token with the error variant"),
        };

        Some(result)
    }
}

fn finish_member<S>(lex: &mut logos::Lexer<AfterMemberDot, S>) {
    debug!("finish_member EXTRAS={:?}", lex.extras);
    lex.extras.current = LexerStateName::AfterVariableToken;
}

#[derive(Debug, Clone, Copy)]
crate enum LexerStateName {
    Top,
    Var,
    AfterMemberDot,
    AfterVariableToken,
}

impl Default for LexerStateName {
    fn default() -> LexerStateName {
        LexerStateName::Top
    }
}

#[derive(Debug, Clone, Default)]
crate struct LexerState {
    current: LexerStateName,
}

impl logos::Extras for LexerState {
    fn on_advance(&mut self) {}
    fn on_whitespace(&mut self, _byte: u8) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
pub struct Span {
    start: usize,
    end: usize,
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

#[derive(new, Debug, Clone, Eq, PartialEq)]
pub struct SpannedToken<'source> {
    crate span: Span,
    crate slice: &'source str,
    crate token: Token,
}

impl SpannedToken<'source> {
    crate fn to_string(&self) -> String {
        self.slice.to_string()
    }

    crate fn as_slice(&self) -> &str {
        self.slice
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Token {
    Variable,
    PathDot,
    Member,
    Num,
    SQString,
    DQString,
    UnitsNum,
    Dollar,
    Bare,
    Pipe,
    OpenBrace,
    CloseBrace,
    OpenParen,
    CloseParen,
    OpGt,
    OpLt,
    OpGte,
    OpLte,
    OpEq,
    OpNeq,
    Dash,
    DashDash,
    Whitespace,
}

// #[derive(Debug, Clone, Eq, PartialEq)]
// crate enum Token<'source> {
//     Top(SpannedToken<'source, TopToken>),
//     Var(SpannedToken<'source, VariableToken>),
//     Dot(SpannedToken<'source, &'source str>),
//     Member(SpannedToken<'source, &'source str>),
//     Whitespace(SpannedToken<'source, &'source str>),
// }

crate struct Lexer<'source> {
    lexer: logos::Lexer<TopToken, &'source str>,
    first: bool,
    whitespace: bool, // state: LexerState
}

impl Lexer<'source> {
    crate fn new(source: &str, whitespace: bool) -> Lexer<'_> {
        Lexer {
            first: true,
            lexer: logos::Logos::lexer(source),
            whitespace
            // state: LexerState::default(),
        }
    }
}

impl Iterator for Lexer<'source> {
    type Item = Result<(usize, SpannedToken<'source>, usize), ShellError>;
    // type Item = Result<Token<'source>, ShellError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.first {
            self.first = false;

            match self.lexer.token {
                TopToken::Error => {
                    return Some(Err(lex_error(&self.lexer.range(), self.lexer.source)))
                }
                TopToken::Whitespace if !self.whitespace => return self.next(),
                other => {
                    return spanned(other.to_token()?, self.lexer.slice(), &self.lexer.range())
                }
            }
        } else {
            debug!("STATE={:?}", self.lexer.extras);

            match self.lexer.extras.current {
                LexerStateName::Top => {
                    let (lexer, range, slice, token) = advance::<TopToken>(self.lexer.clone());
                    self.lexer = lexer;

                    match token {
                        TopToken::Error => return Some(Err(lex_error(&range, self.lexer.source))),
                        TopToken::Whitespace if !self.whitespace => return self.next(),
                        other => return spanned(other.to_token()?, slice, &range),
                    }
                }

                LexerStateName::AfterMemberDot => {
                    let (lexer, range, slice, token) =
                        advance::<AfterMemberDot>(self.lexer.clone());
                    self.lexer = lexer;

                    match token {
                        AfterMemberDot::Error => {
                            return Some(Err(lex_error(&range, self.lexer.source)))
                        }
                        AfterMemberDot::Whitespace if !self.whitespace => self.next(),
                        other => return spanned(other.to_token()?, slice, &range),
                    }
                }

                LexerStateName::AfterVariableToken => {
                    let (lexer, range, slice, token) =
                        advance::<AfterVariableToken>(self.lexer.clone());
                    self.lexer = lexer;

                    match token {
                        AfterVariableToken::Error => {
                            return Some(Err(lex_error(&range, self.lexer.source)))
                        }
                        AfterVariableToken::Whitespace if !self.whitespace => self.next(),
                        other => return spanned(other.to_token()?, slice, &range),
                    }
                }

                LexerStateName::Var => {
                    let (lexer, range, slice, token) = advance::<VariableToken>(self.lexer.clone());
                    self.lexer = lexer;

                    match token {
                        VariableToken::Error => {
                            return Some(Err(lex_error(&range, self.lexer.source)))
                        }
                        other => return spanned(other.to_token()?, slice, &range),
                    }
                }
            }
        }
    }
}

fn lex_error(range: &Range<usize>, source: &str) -> ShellError {
    use language_reporting::*;

    ShellError::diagnostic(
        Diagnostic::new(Severity::Error, "Lex error")
            .with_label(Label::new_primary(Span::new(range))),
        source.to_string(),
    )
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
        let lex = Lexer::new(source, false);
        let mut current = 0;

        let expected_tokens: Vec<SpannedToken> = tokens
            .iter()
            .filter_map(|token_desc| {
                debug!("{:?}", token_desc);

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

        let actual_tokens = actual_tokens.unwrap();

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
                TokenDesc::Top(tok) => {
                    SpannedToken::new(Span::new(range), self.source, tok.to_token().unwrap())
                }
                TokenDesc::Var(tok) => {
                    SpannedToken::new(Span::new(range), self.source, tok.to_token().unwrap())
                }
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
            tokens![ Bare("open") SP Bare("input2") "???." Member("json") SP "|" SP Bare("from-json") SP "|" SP Bare("select") SP Bare("glossary") ],
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
