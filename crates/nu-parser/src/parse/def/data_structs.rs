use nu_protocol::{NamedType, PositionalType, SyntaxShape};
use nu_source::Span;

pub type Description = String;
#[derive(Clone, new)]
pub struct Parameter {
    pub pos_type: PositionalType,
    pub desc: Option<Description>,
    pub span: Span,
}

impl Parameter {
    pub fn error() -> Parameter {
        Parameter::new(
            PositionalType::optional("Internal Error", SyntaxShape::Any),
            Some(
                "Wanted to parse a parameter, but no input present. Please report this error!"
                    .to_string(),
            ),
            Span::unknown(),
        )
    }
}

#[derive(Clone, Debug, new)]
pub struct Flag {
    pub long_name: String,
    pub named_type: NamedType,
    pub desc: Option<Description>,
    pub span: Span,
}

impl Flag {
    pub fn error() -> Flag {
        Flag::new(
            "Internal Error".to_string(),
            NamedType::Switch(None),
            Some(
                "Wanted to parse a flag, but no input present. Please report this error!"
                    .to_string(),
            ),
            Span::unknown(),
        )
    }
}
