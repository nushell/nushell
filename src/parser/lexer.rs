use crate::errors::ShellError;
use derive_new::new;
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
    Size,

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

    #[regex = r"\s+"]
    Whitespace,
}

fn start_variable<S>(lex: &mut logos::Lexer<TopToken, S>) {
    println!("start_variable EXTRAS={:?}", lex.extras);
    lex.extras.current = LexerStateName::Var;
}

fn end_bare_variable<S>(lex: &mut logos::Lexer<TopToken, S>) {
    println!("end_variable EXTRAS={:?}", lex.extras);
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

fn end_variable<S>(lex: &mut logos::Lexer<VariableToken, S>) {
    println!("end_variable EXTRAS={:?}", lex.extras);
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

fn start_member<S>(lex: &mut logos::Lexer<AfterVariableToken, S>) {
    println!("start_variable EXTRAS={:?}", lex.extras);
    lex.extras.current = LexerStateName::AfterMemberDot;
}

fn terminate_variable<S>(lex: &mut logos::Lexer<AfterVariableToken, S>) {
    println!("terminate_variable EXTRAS={:?}", lex.extras);
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

fn finish_member<S>(lex: &mut logos::Lexer<AfterMemberDot, S>) {
    println!("finish_member EXTRAS={:?}", lex.extras);
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

#[derive(new, Debug, Clone, Eq, PartialEq)]
crate struct SpannedToken<'source, T> {
    span: std::ops::Range<usize>,
    slice: &'source str,
    token: T,
}

#[derive(Debug, Clone, Eq, PartialEq)]
crate enum Token<'source> {
    Top(SpannedToken<'source, TopToken>),
    Var(SpannedToken<'source, VariableToken>),
    Dot(SpannedToken<'source, &'source str>),
    Member(SpannedToken<'source, &'source str>),
    Whitespace(SpannedToken<'source, &'source str>),
}

impl Token<'source> {
    crate fn range(&self) -> &Range<usize> {
        match self {
            Token::Top(spanned) => &spanned.span,
            Token::Var(spanned) => &spanned.span,
            Token::Dot(spanned) => &spanned.span,
            Token::Member(spanned) => &spanned.span,
            Token::Whitespace(spanned) => &spanned.span,
        }
    }

    crate fn slice(&self) -> &str {
        match self {
            Token::Top(spanned) => spanned.slice,
            Token::Var(spanned) => spanned.slice,
            Token::Dot(spanned) => spanned.slice,
            Token::Member(spanned) => spanned.slice,
            Token::Whitespace(spanned) => spanned.slice,
        }
    }
}

crate struct Lexer<'source> {
    lexer: logos::Lexer<TopToken, &'source str>,
    first: bool,
    // state: LexerState,
}

impl Lexer<'source> {
    crate fn new(source: &str) -> Lexer<'_> {
        Lexer {
            first: true,
            lexer: logos::Logos::lexer(source),
            // state: LexerState::default(),
        }
    }
}

impl Iterator for Lexer<'source> {
    type Item = Result<Token<'source>, ShellError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.first {
            self.first = false;

            match self.lexer.token {
                TopToken::END => None,
                TopToken::Whitespace => Some(Ok(Token::Whitespace(SpannedToken::new(
                    self.lexer.range(),
                    self.lexer.slice(),
                    self.lexer.slice(),
                )))),
                _ => {
                    let token = Token::Top(SpannedToken::new(
                        self.lexer.range(),
                        self.lexer.slice(),
                        self.lexer.token,
                    ));
                    Some(Ok(token))
                }
            }
        } else {
            println!("STATE={:?}", self.lexer.extras);

            match self.lexer.extras.current {
                LexerStateName::Top => {
                    let (lexer, range, slice, token) = advance::<TopToken>(self.lexer.clone());
                    self.lexer = lexer;

                    match token {
                        TopToken::END => None,
                        TopToken::Whitespace => Some(Ok(Token::Whitespace(SpannedToken::new(
                            range, slice, slice,
                        )))),
                        other => {
                            let token = Token::Top(SpannedToken::new(range, slice, other));
                            Some(Ok(token))
                        }
                    }
                }

                LexerStateName::AfterMemberDot => {
                    let (lexer, range, slice, token) =
                        advance::<AfterMemberDot>(self.lexer.clone());
                    self.lexer = lexer;

                    match token {
                        AfterMemberDot::END => None,
                        AfterMemberDot::Error => {
                            Some(Err(ShellError::string(&format!("Lex error at {}", slice))))
                        }
                        AfterMemberDot::Whitespace => Some(Ok(Token::Whitespace(
                            SpannedToken::new(range, slice, slice),
                        ))),
                        AfterMemberDot::Member => {
                            Some(Ok(Token::Member(SpannedToken::new(range, slice, slice))))
                        }
                    }
                }

                LexerStateName::AfterVariableToken => {
                    let (lexer, range, slice, token) =
                        advance::<AfterVariableToken>(self.lexer.clone());
                    self.lexer = lexer;

                    match token {
                        AfterVariableToken::END => None,
                        AfterVariableToken::Error => {
                            Some(Err(ShellError::string(&format!("Lex error at {}", slice))))
                        }
                        AfterVariableToken::Whitespace => Some(Ok(Token::Whitespace(
                            SpannedToken::new(range, slice, slice),
                        ))),
                        AfterVariableToken::Dot => {
                            Some(Ok(Token::Dot(SpannedToken::new(range, slice, slice))))
                        }
                    }
                }

                LexerStateName::Var => {
                    let (lexer, range, slice, token) = advance::<VariableToken>(self.lexer.clone());
                    self.lexer = lexer;

                    match token {
                        VariableToken::END => None,
                        other => {
                            let token = Token::Var(SpannedToken::new(range, slice, other));
                            Some(Ok(token))
                        }
                    }
                }
            }
        }
    }
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
    use logos::Logos;
    use pretty_assertions::assert_eq;

    fn assert_lex(source: &str, tokens: &[TestToken<'_>]) {
        let lex = Lexer::new(source);
        let mut current = 0;

        let expected_tokens: Vec<Token> = tokens
            .iter()
            .map(|token_desc| {
                println!("{:?}", token_desc);

                let len = token_desc.source.len();
                let range = current..(current + len);
                let token = token_desc.to_token(range);

                current = current + len;

                token
            })
            .collect();

        let actual_tokens: Result<Vec<Token>, _> = lex
            .map(|i| {
                println!("{:?}", i);
                i
            })
            .collect();

        let actual_tokens = actual_tokens.unwrap();

        assert_eq!(actual_tokens, expected_tokens);
    }

    #[derive(Debug)]
    enum TokenDesc {
        Ws,
        Member,
        Top(TopToken),
        Var(VariableToken),
    }

    #[derive(Debug, new)]
    struct TestToken<'source> {
        desc: TokenDesc,
        source: &'source str,
    }

    impl TestToken<'source> {
        fn to_token(&self, span: std::ops::Range<usize>) -> Token {
            match self.desc {
                TokenDesc::Top(TopToken::Dot) => {
                    Token::Dot(SpannedToken::new(span, self.source, "."))
                }
                TokenDesc::Top(tok) => Token::Top(SpannedToken::new(span, self.source, tok)),
                TokenDesc::Var(tok) => Token::Var(SpannedToken::new(span, self.source, tok)),
                TokenDesc::Member => {
                    Token::Member(SpannedToken::new(span, self.source, self.source))
                }
                TokenDesc::Ws => {
                    Token::Whitespace(SpannedToken::new(span, self.source, self.source))
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
        assert_lex("$var.bar", tokens![ "$" Var("var") "." Member("bar") ]);
        assert_lex("$it.bar", tokens![ "$" Var("it") "." Member("bar") ]);
        assert_lex("$var. bar", tokens![ "$" Var("var") "." SP Member("bar") ]);
        assert_lex("$it. bar", tokens![ "$" Var("it") "." SP Member("bar") ]);
    }

    #[test]
    fn test_tokenize_operator() {
        assert_lex(
            "$it.cpu > 10",
            tokens![ "$" Var("it") "." Member("cpu") SP ">" SP Num("10") ],
        );

        assert_lex(
            "$it.cpu < 10",
            tokens![ "$" Var("it") "." Member("cpu") SP "<" SP Num("10") ],
        );

        assert_lex(
            "$it.cpu >= 10",
            tokens![ "$" Var("it") "." Member("cpu") SP ">=" SP Num("10") ],
        );

        assert_lex(
            "$it.cpu <= 10",
            tokens![ "$" Var("it") "." Member("cpu") SP "<=" SP Num("10") ],
        );

        assert_lex(
            "$it.cpu == 10",
            tokens![ "$" Var("it") "." Member("cpu") SP "==" SP Num("10") ],
        );

        assert_lex(
            "$it.cpu != 10",
            tokens![ "$" Var("it") "." Member("cpu") SP "!=" SP Num("10") ],
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
            tokens![ Bare("ls") SP "|" SP Bare("where") SP "{" SP "$" Var("it") "." Member("cpu") SP ">" SP Num("10") SP "}" ],
        );

        assert_lex(
            "open input2.json | from-json | select glossary",
            tokens![ Bare("open") SP Bare("input2") "." Member("json") SP "|" SP Bare("from-json") SP "|" SP Bare("select") SP Bare("glossary") ],
        );
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
            "." => TopToken::Dot,
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
