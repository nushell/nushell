use crate::parser::parse2::{operator::*, span::*, tokens::*};
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

named!(integer( NomSpan ) -> Token,
    do_parse!(
            l: position!()
        >>  neg: opt!(tag!("-"))
        >>  num: digit1
        >>  r: position!()
        >>  (Spanned::from_nom(RawToken::Integer(int(num.fragment.0, neg)), l, r))
    )
);

named!(operator( NomSpan ) -> Token,
    alt!(
        gte | lte | neq | gt | lt | eq
    )
);

named!(dq_string( NomSpan ) -> Token,
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

named!(sq_string( NomSpan ) -> Token,
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

fn int<T>(frag: &str, neg: Option<T>) -> i64 {
    let int = FromStr::from_str(frag).unwrap();

    match neg {
        None => int,
        Some(_) => int * -1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer() {
        assert_eq!(
            integer(NomSpan::new(CompleteStr("123"))).unwrap().1,
            Spanned::from_item(RawToken::Integer(123), (0, 3))
        );

        assert_eq!(
            integer(NomSpan::new(CompleteStr("-123"))).unwrap().1,
            Spanned::from_item(RawToken::Integer(-123), (0, 4))
        );
    }

    #[test]
    fn test_operator() {
        assert_eq!(
            operator(NomSpan::new(CompleteStr(">"))).unwrap().1,
            Spanned::from_item(RawToken::Operator(Operator::GreaterThan), (0, 1))
        );

        assert_eq!(
            operator(NomSpan::new(CompleteStr(">="))).unwrap().1,
            Spanned::from_item(RawToken::Operator(Operator::GreaterThanOrEqual), (0, 2))
        );

        assert_eq!(
            operator(NomSpan::new(CompleteStr("<"))).unwrap().1,
            Spanned::from_item(RawToken::Operator(Operator::LessThan), (0, 1))
        );

        assert_eq!(
            operator(NomSpan::new(CompleteStr("<="))).unwrap().1,
            Spanned::from_item(RawToken::Operator(Operator::LessThanOrEqual), (0, 2))
        );

        assert_eq!(
            operator(NomSpan::new(CompleteStr("=="))).unwrap().1,
            Spanned::from_item(RawToken::Operator(Operator::Equal), (0, 2))
        );

        assert_eq!(
            operator(NomSpan::new(CompleteStr("!="))).unwrap().1,
            Spanned::from_item(RawToken::Operator(Operator::NotEqual), (0, 2))
        );
    }

    #[test]
    fn test_string() {
        assert_eq!(
            dq_string(NomSpan::new(CompleteStr(r#""hello world""#)))
                .unwrap()
                .1,
            Spanned::from_item(RawToken::String(Span::from((1, 12))), (0, 13))
        );

        assert_eq!(
            sq_string(NomSpan::new(CompleteStr(r#"'hello world'"#)))
                .unwrap()
                .1,
            Spanned::from_item(RawToken::String(Span::from((1, 12))), (0, 13))
        );
    }
}
