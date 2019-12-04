use crate::syntax_shape::SyntaxShape;
use crate::type_shape::Type;
use indexmap::IndexMap;
use nu_source::{b, DebugDocBuilder, PrettyDebug, PrettyDebugWithSource};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum NamedType {
    Switch,
    Mandatory(SyntaxShape),
    Optional(SyntaxShape),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PositionalType {
    Mandatory(String, SyntaxShape),
    Optional(String, SyntaxShape),
}

impl PrettyDebug for PositionalType {
    fn pretty(&self) -> DebugDocBuilder {
        match self {
            PositionalType::Mandatory(string, shape) => {
                b::description(string) + b::delimit("(", shape.pretty(), ")").into_kind().group()
            }
            PositionalType::Optional(string, shape) => {
                b::description(string)
                    + b::operator("?")
                    + b::delimit("(", shape.pretty(), ")").into_kind().group()
            }
        }
    }
}

impl PositionalType {
    pub fn mandatory(name: &str, ty: SyntaxShape) -> PositionalType {
        PositionalType::Mandatory(name.to_string(), ty)
    }

    pub fn mandatory_any(name: &str) -> PositionalType {
        PositionalType::Mandatory(name.to_string(), SyntaxShape::Any)
    }

    pub fn mandatory_block(name: &str) -> PositionalType {
        PositionalType::Mandatory(name.to_string(), SyntaxShape::Block)
    }

    pub fn optional(name: &str, ty: SyntaxShape) -> PositionalType {
        PositionalType::Optional(name.to_string(), ty)
    }

    pub fn optional_any(name: &str) -> PositionalType {
        PositionalType::Optional(name.to_string(), SyntaxShape::Any)
    }

    pub fn name(&self) -> &str {
        match self {
            PositionalType::Mandatory(s, _) => s,
            PositionalType::Optional(s, _) => s,
        }
    }

    pub fn syntax_type(&self) -> SyntaxShape {
        match *self {
            PositionalType::Mandatory(_, t) => t,
            PositionalType::Optional(_, t) => t,
        }
    }
}

type Description = String;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Signature {
    pub name: String,
    pub usage: String,
    pub positional: Vec<(PositionalType, Description)>,
    pub rest_positional: Option<(SyntaxShape, Description)>,
    pub named: IndexMap<String, (NamedType, Description)>,
    pub yields: Option<Type>,
    pub input: Option<Type>,
    pub is_filter: bool,
}

impl PrettyDebugWithSource for Signature {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        b::typed(
            "signature",
            b::description(&self.name)
                + b::preceded(
                    b::space(),
                    b::intersperse(
                        self.positional
                            .iter()
                            .map(|(ty, _)| ty.pretty_debug(source)),
                        b::space(),
                    ),
                ),
        )
    }
}

impl Signature {
    pub fn new(name: impl Into<String>) -> Signature {
        Signature {
            name: name.into(),
            usage: String::new(),
            positional: vec![],
            rest_positional: None,
            named: IndexMap::new(),
            is_filter: false,
            yields: None,
            input: None,
        }
    }

    pub fn build(name: impl Into<String>) -> Signature {
        Signature::new(name.into())
    }

    pub fn desc(mut self, usage: impl Into<String>) -> Signature {
        self.usage = usage.into();
        self
    }

    pub fn required(
        mut self,
        name: impl Into<String>,
        ty: impl Into<SyntaxShape>,
        desc: impl Into<String>,
    ) -> Signature {
        self.positional.push((
            PositionalType::Mandatory(name.into(), ty.into()),
            desc.into(),
        ));

        self
    }

    pub fn optional(
        mut self,
        name: impl Into<String>,
        ty: impl Into<SyntaxShape>,
        desc: impl Into<String>,
    ) -> Signature {
        self.positional.push((
            PositionalType::Optional(name.into(), ty.into()),
            desc.into(),
        ));

        self
    }

    pub fn named(
        mut self,
        name: impl Into<String>,
        ty: impl Into<SyntaxShape>,
        desc: impl Into<String>,
    ) -> Signature {
        self.named
            .insert(name.into(), (NamedType::Optional(ty.into()), desc.into()));

        self
    }

    pub fn required_named(
        mut self,
        name: impl Into<String>,
        ty: impl Into<SyntaxShape>,
        desc: impl Into<String>,
    ) -> Signature {
        self.named
            .insert(name.into(), (NamedType::Mandatory(ty.into()), desc.into()));

        self
    }

    pub fn switch(mut self, name: impl Into<String>, desc: impl Into<String>) -> Signature {
        self.named
            .insert(name.into(), (NamedType::Switch, desc.into()));

        self
    }

    pub fn filter(mut self) -> Signature {
        self.is_filter = true;
        self
    }

    pub fn rest(mut self, ty: SyntaxShape, desc: impl Into<String>) -> Signature {
        self.rest_positional = Some((ty, desc.into()));
        self
    }

    pub fn yields(mut self, ty: Type) -> Signature {
        self.yields = Some(ty);
        self
    }

    pub fn input(mut self, ty: Type) -> Signature {
        self.input = Some(ty);
        self
    }
}
