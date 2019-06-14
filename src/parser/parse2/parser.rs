#![allow(unused)]

use crate::parser::parse2::{flag::*, operator::*, span::*, token_tree::*, tokens::*, unit::*};
use nom;
use nom::dbg;
use nom::types::CompleteStr;
use nom::*;
use nom_locate::{position, LocatedSpan};
use std::str::FromStr;

type NomSpan<'a> = LocatedSpan<CompleteStr<'a>>;

macro_rules! operator {
    ($name:tt : $token:tt ) => {
        named!($name( NomSpan ) -> Token,
            do_parse!(
                l: position!()
                    >> t: tag!(stringify!($token))
                    >> r: position!()
                    >> (Spanned::from_nom(RawToken::Operator(Operator::from_str(t.fragment.0).unwrap()), l, r))
            )
        );
    };
}

operator! { gt:  >  }
operator! { lt:  <  }
operator! { gte: >= }
operator! { lte: <= }
operator! { eq:  == }
operator! { neq: != }

named!(pub raw_integer( NomSpan ) -> Spanned<i64>,
    do_parse!(
            l: position!()
        >>  neg: opt!(tag!("-"))
        >>  num: digit1
        >>  r: position!()
        >>  (Spanned::from_nom(int(num.fragment.0, neg), l, r))
    )
);

named!(pub integer( NomSpan ) -> Token,
    do_parse!(
            int: raw_integer
        >>  (int.map(|i| RawToken::Integer(i)))
    )
);

named!(pub operator( NomSpan ) -> Token,
    alt!(
        gte | lte | neq | gt | lt | eq
    )
);

named!(pub dq_string( NomSpan ) -> Token,
    do_parse!(
            l: position!()
        >>  char!('"')
        >>  l1: position!()
        >>  many0!(none_of!("\""))
        >>  r1: position!()
        >>  char!('"')
        >>  r: position!()
        >>  (Spanned::from_nom(RawToken::String(Span::from((l1, r1))), l, r))
    )
);

named!(pub sq_string( NomSpan ) -> Token,
    do_parse!(
            l: position!()
        >>  char!('\'')
        >>  l1: position!()
        >>  many0!(none_of!("'"))
        >>  r1: position!()
        >>  char!('\'')
        >>  r: position!()
        >>  (Spanned::from_nom(RawToken::String(Span::from((l1, r1))), l, r))
    )
);

named!(pub string( NomSpan ) -> Token,
    alt!(sq_string | dq_string)
);

named!(pub bare( NomSpan ) -> Token,
    do_parse!(
            l: position!()
        >>  take_while1!(is_start_bare_char)
        >>  take_while!(is_bare_char)
        >>  r: position!()
        >>  (Spanned::from_nom(RawToken::Bare, l, r))
    )
);

named!(pub var( NomSpan ) -> Token,
    do_parse!(
            l: position!()
        >>  tag!("$")
        >>  bare: identifier
        >>  r: position!()
        >>  (Spanned::from_nom(RawToken::Variable(bare.span), l, r))
    )
);

named!(pub identifier( NomSpan ) -> Token,
    do_parse!(
            l: position!()
        >>  take_while1!(is_id_start)
        >>  take_while!(is_id_continue)
        >>  r: position!()
        >>  (Spanned::from_nom(RawToken::Identifier, l, r))
    )
);

named!(pub flag( NomSpan ) -> Token,
    do_parse!(
            l: position!()
        >>  tag!("--")
        >>  bare: bare
        >>  r: position!()
        >>  (Spanned::from_nom(RawToken::Flag(Flag::Longhand, bare.span), l, r))
    )
);

named!(pub shorthand( NomSpan ) -> Token,
    do_parse!(
            l: position!()
        >>  tag!("-")
        >>  bare: bare
        >>  r: position!()
        >>  (Spanned::from_nom(RawToken::Flag(Flag::Shorthand, bare.span), l, r))
    )
);

named!(pub raw_unit( NomSpan ) -> Spanned<Unit>,
    do_parse!(
            l: position!()
        >>  unit: alt!(tag!("B") | tag!("KB") | tag!("MB") | tag!("GB") | tag!("TB") | tag!("PB"))
        >>  r: position!()
        >>  (Spanned::from_nom(Unit::from(unit.fragment.0), l, r))
    )
);

named!(pub size( NomSpan ) -> Token,
    do_parse!(
            l: position!()
        >>  int: raw_integer
        >>  unit: raw_unit
        >>  r: position!()
        >>  (Spanned::from_nom(RawToken::Size(int.item, unit.item), l, r))
    )
);

// named!(pub unit_num( NomSpan ) -> Token,
//     do_parse!(
//             l: position!()
//         >>
//     )
// )

named!(pub leaf( NomSpan ) -> Token,
    alt!(size | integer | string | operator | flag | shorthand | var | bare)
);

named!(pub leaf_node( NomSpan ) -> TokenNode,
    do_parse!(
            leaf: leaf
        >>  (TokenNode::Token(leaf))
    )
);

named!(pub delimited_paren( NomSpan ) -> TokenNode,
    do_parse!(
            l: position!()
        >>  items: delimited!(
                char!('('),
                delimited!(space0, separated_list!(space1, node), space0),
                char!(')')
            )
        >>  r: position!()
        >>  (TokenNode::Delimited(Spanned::from_nom(DelimitedNode::new(Delimiter::Paren, items), l, r)))
    )
);

named!(pub path( NomSpan ) -> TokenNode,
    do_parse!(
            l: position!()
        >>  head: node1
        >>  tag!(".")
        >>  tail: separated_list!(tag!("."), alt!(identifier | string))
        >>  r: position!()
        >>  (TokenNode::Path(Spanned::from_nom(PathNode::new(Box::new(head), tail.into_iter().map(TokenNode::Token).collect()), l, r)))
    )
);

named!(pub node1( NomSpan ) -> TokenNode,
    alt!(leaf_node | delimited_paren)
);

named!(pub node( NomSpan ) -> TokenNode,
    alt!(path | leaf_node | delimited_paren)
);

fn int<T>(frag: &str, neg: Option<T>) -> i64 {
    let int = FromStr::from_str(frag).unwrap();

    match neg {
        None => int,
        Some(_) => int * -1,
    }
}

fn is_start_bare_char(c: char) -> bool {
    match c {
        _ if c.is_alphabetic() => true,
        '.' => true,
        '\\' => true,
        '/' => true,
        '_' => true,
        '-' => true,
        _ => false,
    }
}

fn is_bare_char(c: char) -> bool {
    match c {
        _ if c.is_alphanumeric() => true,
        ':' => true,
        '.' => true,
        '\\' => true,
        '/' => true,
        '_' => true,
        '-' => true,
        _ => false,
    }
}

fn is_id_start(c: char) -> bool {
    unicode_xid::UnicodeXID::is_xid_start(c)
}

fn is_id_continue(c: char) -> bool {
    unicode_xid::UnicodeXID::is_xid_continue(c)
        || match c {
            '-' => true,
            '?' => true,
            '!' => true,
            _ => false,
        }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse2::token_tree_builder::TokenTreeBuilder as b;
    use crate::parser::parse2::token_tree_builder::{CurriedToken, TokenTreeBuilder};
    use nom_trace::{print_trace, reset_trace};
    use pretty_assertions::assert_eq;

    macro_rules! assert_leaf {
        (parsers [ $($name:tt)* ] $input:tt -> $left:tt .. $right:tt { $kind:tt $parens:tt } ) => {
            $(
                assert_eq!(
                    apply($name, $input),
                    token(RawToken::$kind $parens, $left, $right)
                );
            )*

            assert_eq!(
                apply(leaf, $input),
                token(RawToken::$kind $parens, $left, $right)
            );

            assert_eq!(
                apply(leaf_node, $input),
                TokenNode::Token(token(RawToken::$kind $parens, $left, $right))
            );

            assert_eq!(
                apply(node, $input),
                TokenNode::Token(token(RawToken::$kind $parens, $left, $right))
            );
        };

        (parsers [ $($name:tt)* ] $input:tt -> $left:tt .. $right:tt { $kind:tt } ) => {
            $(
                assert_eq!(
                    apply($name, $input),
                    token(RawToken::$kind, $left, $right)
                );
            )*
        }
    }

    #[test]
    fn test_integer() {
        assert_leaf! {
            parsers [ integer ]
            "123" -> 0..3 { Integer(123) }
        }

        assert_leaf! {
            parsers [ integer ]
            "-123" -> 0..4 { Integer(-123) }
        }
    }

    #[test]
    fn test_size() {
        assert_leaf! {
            parsers [ size ]
            "123MB" -> 0..5 { Size(123, Unit::MB) }
        }

        assert_leaf! {
            parsers [ size ]
            "10GB" -> 0..4 { Size(10, Unit::GB) }
        }
    }

    #[test]
    fn test_operator() {
        assert_leaf! {
            parsers [ operator ]
            ">" -> 0..1 { Operator(Operator::GreaterThan) }
        }

        assert_leaf! {
            parsers [ operator ]
            ">=" -> 0..2 { Operator(Operator::GreaterThanOrEqual) }
        }

        assert_leaf! {
            parsers [ operator ]
            "<" -> 0..1 { Operator(Operator::LessThan) }
        }

        assert_leaf! {
            parsers [ operator ]
            "<=" -> 0..2 { Operator(Operator::LessThanOrEqual) }
        }

        assert_leaf! {
            parsers [ operator ]
            "==" -> 0..2 { Operator(Operator::Equal) }
        }

        assert_leaf! {
            parsers [ operator ]
            "!=" -> 0..2 { Operator(Operator::NotEqual) }
        }
    }

    #[test]
    fn test_string() {
        assert_leaf! {
            parsers [ string dq_string ]
            r#""hello world""# -> 0..13 { String(span(1, 12)) }
        }

        assert_leaf! {
            parsers [ string sq_string ]
            r"'hello world'" -> 0..13 { String(span(1, 12)) }
        }
    }

    #[test]
    fn test_bare() {
        assert_leaf! {
            parsers [ bare ]
            "hello" -> 0..5 { Bare }
        }

        assert_leaf! {
            parsers [ bare ]
            "chrome.exe" -> 0..10 { Bare }
        }

        assert_leaf! {
            parsers [ bare ]
            r"C:\windows\system.dll" -> 0..21 { Bare }
        }

        assert_leaf! {
            parsers [ bare ]
            r"C:\Code\-testing\my_tests.js" -> 0..28 { Bare }
        }
    }

    #[test]
    fn test_flag() {
        assert_leaf! {
            parsers [ flag ]
            "--hello" -> 0..7 { Flag(Flag::Longhand, span(2, 7)) }
        }

        assert_leaf! {
            parsers [ flag ]
            "--hello-world" -> 0..13 { Flag(Flag::Longhand, span(2, 13)) }
        }
    }

    #[test]
    fn test_shorthand() {
        assert_leaf! {
            parsers [ shorthand ]
            "-alt" -> 0..4 { Flag(Flag::Shorthand, span(1, 4)) }
        }
    }

    #[test]
    fn test_variable() {
        assert_leaf! {
            parsers [ var ]
            "$it" -> 0..3 { Variable(span(1, 3)) }
        }

        assert_leaf! {
            parsers [ var ]
            "$name" -> 0..5 { Variable(span(1, 5)) }
        }
    }

    #[test]
    fn test_delimited() {
        assert_eq!(apply(node, "(abc)"), build(b::parens(vec![b::bare("abc")])));

        assert_eq!(
            apply(node, "(  abc  )"),
            build(b::parens(vec![b::ws("  "), b::bare("abc"), b::ws("  ")]))
        );

        assert_eq!(
            apply(node, "(  abc def )"),
            build(b::parens(vec![
                b::ws("  "),
                b::bare("abc"),
                b::sp(),
                b::bare("def"),
                b::sp()
            ]))
        );

        assert_eq!(
            apply(node, "(  abc def 123 456GB )"),
            build(b::parens(vec![
                b::ws("  "),
                b::bare("abc"),
                b::sp(),
                b::bare("def"),
                b::sp(),
                b::int(123),
                b::sp(),
                b::size(456, "GB"),
                b::sp()
            ]))
        );
    }

    #[test]
    fn test_path() {
        assert_eq!(
            apply(node, "$it.print"),
            build(b::path(b::var("it"), vec![b::ident("print")]))
        );

        assert_eq!(
            apply(node, "$head.part1.part2"),
            build(b::path(
                b::var("head"),
                vec![b::ident("part1"), b::ident("part2")]
            ))
        );

        assert_eq!(
            apply(node, "( hello ).world"),
            build(b::path(
                b::parens(vec![b::sp(), b::bare("hello"), b::sp()]),
                vec![b::ident("world")]
            ))
        );

        assert_eq!(
            apply(node, "( hello ).\"world\""),
            build(b::path(
                b::parens(vec![b::sp(), b::bare("hello"), b::sp()],),
                vec![b::string("world")]
            ))
        );
    }

    #[test]
    fn test_nested_path() {
        assert_eq!(
            apply(node, "( $it.is.\"great news\".right yep $yep ).\"world\""),
            build(b::path(
                b::parens(vec![
                    b::sp(),
                    b::path(
                        b::var("it"),
                        vec![b::ident("is"), b::string("great news"), b::ident("right")]
                    ),
                    b::sp(),
                    b::bare("yep"),
                    b::sp(),
                    b::var("yep"),
                    b::sp()
                ]),
                vec![b::string("world")]
            ))
        )
    }

    fn apply<T>(f: impl Fn(NomSpan) -> Result<(NomSpan, T), nom::Err<NomSpan>>, string: &str) -> T {
        match f(NomSpan::new(CompleteStr(string))) {
            Ok(v) => v.1,
            Err(other) => {
                println!("{:?}", other);
                panic!("No dice");
            }
        }
    }

    fn span(left: usize, right: usize) -> Span {
        Span::from((left, right))
    }

    fn delimited(
        delimiter: Delimiter,
        children: Vec<TokenNode>,
        left: usize,
        right: usize,
    ) -> TokenNode {
        let node = DelimitedNode::new(delimiter, children);
        let spanned = Spanned::from_item(node, (left, right));
        TokenNode::Delimited(spanned)
    }

    fn path(head: TokenNode, tail: Vec<Token>, left: usize, right: usize) -> TokenNode {
        let node = PathNode::new(
            Box::new(head),
            tail.into_iter().map(TokenNode::Token).collect(),
        );
        let spanned = Spanned::from_item(node, (left, right));
        TokenNode::Path(spanned)
    }

    fn leaf_token(token: RawToken, left: usize, right: usize) -> TokenNode {
        TokenNode::Token(Spanned::from_item(token, (left, right)))
    }

    fn token(token: RawToken, left: usize, right: usize) -> Token {
        Spanned::from_item(token, (left, right))
    }

    fn build(block: CurriedToken) -> TokenNode {
        let mut builder = TokenTreeBuilder::new();
        block(&mut builder).expect("Expected to build into a token")
    }
}
