use proc_macro2::*;
use quote::{ToTokens, TokenStreamExt, quote, quote_spanned};

pub fn test_cell_path(tokens: TokenStream) -> Result<TokenStream, Error> {
    let path_members = parse_tokens(tokens)?;
    Ok(quote! {
        ::nu_protocol::ast::CellPath {
            members: ::std::vec![#(#path_members),*]
        }
    })
}

pub struct Error {
    span: Span,
    kind: ErrorKind,
}
pub enum ErrorKind {
    EmptyTokenStream,
    ExpectedValue,
    ExpectedModifierOrDot,
    DotWithoutFollowingComponent,
    DuplicateOptional,
    DuplicateInsensitive,
    UnexpectedPunct,
    MissingFinalValue,
}

impl Error {
    pub fn into_compile_error(self) -> TokenStream {
        let span = self.span;
        let msg = match self.kind {
            ErrorKind::EmptyTokenStream => "empty token stream is not allowed",
            ErrorKind::ExpectedValue => "expected group, ident or literal",
            ErrorKind::ExpectedModifierOrDot => "expected ! ? or .",
            ErrorKind::DotWithoutFollowingComponent => "dot without following component",
            ErrorKind::DuplicateOptional => "duplicate ?",
            ErrorKind::DuplicateInsensitive => "duplicate !",
            ErrorKind::UnexpectedPunct => "unexpected punctuation",
            ErrorKind::MissingFinalValue => "missing final path member",
        };

        quote_spanned! { span =>
            compile_error!(#msg)
        }
    }
}

struct PathMember {
    value: PathMemberValue,
    optional: Option<Punct>,
    insensitive: Option<Punct>,
}

impl ToTokens for PathMember {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let PathMember {
            value,
            optional,
            insensitive,
        } = self;

        let ts = match (optional, insensitive) {
            (None, None) => quote! {{
                ::nu_protocol::ast::TestPathMember::from(#value).into_path_member()
            }},
            (Some(_), None) => quote! {{
                let mut path_member = ::nu_protocol::ast::TestPathMember::from(#value).into_path_member();
                path_member.make_optional();
                path_member
            }},
            (None, Some(_)) => quote! {{
                let mut path_member = ::nu_protocol::ast::TestPathMember::from(#value).into_path_member();
                path_member.make_insensitive();
                path_member
            }},
            (Some(_), Some(_)) => quote! {{
                let mut path_member = ::nu_protocol::ast::TestPathMember::from(#value).into_path_member();
                path_member.make_optional();
                path_member.make_insensitive();
                path_member
            }},
        };

        tokens.append_all(ts)
    }
}

enum PathMemberValue {
    Group(Group),
    Ident(Ident),
    Literal(Literal),
}

impl ToTokens for PathMemberValue {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            PathMemberValue::Group(group) => group.to_tokens(tokens),
            PathMemberValue::Ident(ident) => {
                let mut literal = Literal::string(&ident.to_string());
                literal.set_span(ident.span());
                literal.to_tokens(tokens);
            }
            PathMemberValue::Literal(literal) => literal.to_tokens(tokens),
        }
    }
}

fn parse_tokens(tokens: TokenStream) -> Result<Vec<PathMember>, Error> {
    use PathMemberValue as PMV;
    use TokenTree as TT;

    if tokens.is_empty() {
        return Err(Error {
            span: Span::call_site(),
            kind: ErrorKind::EmptyTokenStream,
        });
    }

    let mut tokens = tokens.into_iter().peekable();
    let mut values = Vec::new();
    let mut value = None::<PathMemberValue>;
    let mut optional = None::<Punct>;
    let mut insensitive = None::<Punct>;

    while let Some(tt) = tokens.next() {
        let span = tt.span();
        let err = |kind| Error { span, kind };
        match (tt, &value, &optional, &insensitive) {
            (TT::Group(v), None, None, None) => value = Some(PMV::Group(v)),
            (TT::Ident(v), None, None, None) => value = Some(PMV::Ident(v)),
            (TT::Literal(v), None, None, None) => value = Some(PMV::Literal(v)),

            (TT::Punct(_), None, _, _) => return Err(err(ErrorKind::ExpectedValue)),

            (TT::Group(_) | TT::Ident(_) | TT::Literal(_), Some(_), _, _) => {
                return Err(err(ErrorKind::ExpectedModifierOrDot));
            }

            (TT::Punct(punct), Some(_), _, _) => match punct.as_char() {
                '.' => match tokens.peek() {
                    None => return Err(err(ErrorKind::DotWithoutFollowingComponent)),
                    Some(_) => values.push(PathMember {
                        value: value.take().expect("is some"),
                        optional: optional.take(),
                        insensitive: insensitive.take(),
                    }),
                },

                '?' => match optional {
                    None => optional = Some(punct),
                    Some(_) => return Err(err(ErrorKind::DuplicateOptional)),
                },

                '!' => match insensitive {
                    None => insensitive = Some(punct),
                    Some(_) => return Err(err(ErrorKind::DuplicateInsensitive)),
                },

                _ => return Err(err(ErrorKind::UnexpectedPunct)),
            },

            (TT::Group(_), None, _, _)
            | (TT::Ident(_), None, _, _)
            | (TT::Literal(_), None, _, _) => {
                return Err(err(ErrorKind::ExpectedValue));
            }
        }
    }

    let Some(value) = value.take() else {
        return Err(Error {
            span: Span::call_site(),
            kind: ErrorKind::MissingFinalValue,
        });
    };

    values.push(PathMember {
        value,
        optional,
        insensitive,
    });

    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proc_macro2::TokenStream;

    #[test]
    fn error_empty_token_stream() {
        let Err(err) = parse_tokens(TokenStream::new()) else {
            panic!("expected error");
        };

        assert!(matches!(&err.kind, ErrorKind::EmptyTokenStream));
        let msg = err.into_compile_error().to_string();
        assert!(msg.contains("empty token stream is not allowed"));
    }

    #[test]
    fn error_expected_value() {
        let Err(err) = parse_tokens(quote!(.)) else {
            panic!("expected error");
        };

        assert!(matches!(&err.kind, ErrorKind::ExpectedValue));
        let msg = err.into_compile_error().to_string();
        assert!(msg.contains("expected group, ident or literal"));
    }

    #[test]
    fn error_expected_modifier_or_dot() {
        let Err(err) = parse_tokens(quote!(foo bar)) else {
            panic!("expected error");
        };

        assert!(matches!(&err.kind, ErrorKind::ExpectedModifierOrDot));
        let msg = err.into_compile_error().to_string();
        assert!(msg.contains("expected ! ? or ."));
    }

    #[test]
    fn error_dot_without_following_component() {
        let Err(err) = parse_tokens(quote!(foo.)) else {
            panic!("expected error");
        };

        assert!(matches!(&err.kind, ErrorKind::DotWithoutFollowingComponent));
        let msg = err.into_compile_error().to_string();
        assert!(msg.contains("dot without following component"));
    }

    #[test]
    fn error_duplicate_optional() {
        let Err(err) = parse_tokens(quote!(foo??)) else {
            panic!("expected error");
        };

        assert!(matches!(&err.kind, ErrorKind::DuplicateOptional));
        let msg = err.into_compile_error().to_string();
        assert!(msg.contains("duplicate ?"));
    }

    #[test]
    fn error_duplicate_insensitive() {
        let Err(err) = parse_tokens(quote!(foo!!)) else {
            panic!("expected error");
        };

        assert!(matches!(&err.kind, ErrorKind::DuplicateInsensitive));
        let msg = err.into_compile_error().to_string();
        assert!(msg.contains("duplicate !"));
    }

    #[test]
    fn error_unexpected_punct() {
        let Err(err) = parse_tokens(quote!(foo,)) else {
            panic!("expected error");
        };

        assert!(matches!(&err.kind, ErrorKind::UnexpectedPunct));
        let msg = err.into_compile_error().to_string();
        assert!(msg.contains("unexpected punctuation"));
    }
}
