use indexmap::IndexMap;
use parking_lot::Mutex;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::sync::Arc;

use nu_source::{b, DebugDocBuilder, HasSpan, PrettyDebug, PrettyDebugWithSource, Span, Tag};

pub trait SignatureRegistry: Debug {
    fn has(&self, name: &str) -> bool;
    fn get(&self, name: &str) -> Option<nu_protocol::Signature>;
    fn clone_box(&self) -> Box<dyn SignatureRegistry>;
}

impl SignatureRegistry for Box<dyn SignatureRegistry> {
    fn has(&self, name: &str) -> bool {
        (&**self).has(name)
    }
    fn get(&self, name: &str) -> Option<nu_protocol::Signature> {
        (&**self).get(name)
    }
    fn clone_box(&self) -> Box<dyn SignatureRegistry> {
        (&**self).clone_box()
    }
}

pub enum RangeInclusion {
    Inclusive,
    Exclusive,
}

pub struct RangeType {
    from: (Type, RangeInclusion),
    to: (Type, RangeInclusion),
}

// #[derive(Debug, Clone)]
// pub enum SyntaxShape {
//     /// Any syntactic form is allowed
//     Any,
//     /// One of the following possible shapes, in order
//     OneOf(Vec<SyntaxShape>),
//     /// Strings and string-like bare words are allowed
//     String,
//     /// Values that can be the right hand side of a '.'
//     Member,
//     /// A dotted path to navigate the table
//     ColumnPath,
//     /// Only a numeric (integer or decimal) value is allowed
//     Number,
//     /// A range is allowed (eg, `1..3`)
//     Range,
//     /// Only an integer value is allowed
//     Int,
//     /// A filepath is allowed
//     Path,
//     /// A glob pattern is allowed, eg `foo*`
//     Pattern,
//     /// A block is allowed, eg `{start this thing}`
//     Block,
//     /// A table is allowed, eg `[first second]`
//     Table,
//     /// A unit value is allowed, eg `10kb`
//     Unit,
//     /// An operator
//     Operator,
// }

// type Description = String;

// #[derive(Clone)]
// pub enum NamedType {
//     /// A flag without any associated argument. eg) `foo --bar, foo -b`
//     Switch(Option<char>),
//     /// A mandatory flag, with associated argument. eg) `foo --required xyz, foo -r xyz`
//     Mandatory(Option<char>, SyntaxShape),
//     /// An optional flag, with associated argument. eg) `foo --optional abc, foo -o abc`
//     Optional(Option<char>, SyntaxShape),
// }

// impl NamedType {
//     pub fn get_short(&self) -> Option<char> {
//         match self {
//             NamedType::Switch(s) => *s,
//             NamedType::Mandatory(s, _) => *s,
//             NamedType::Optional(s, _) => *s,
//         }
//     }
// }

pub enum Column {
    String(String),
    Value,
}

pub struct Row {
    map: BTreeMap<Column, Type>,
}

pub enum Type {
    /// A value which has no value
    Nothing,
    /// An integer-based value
    Int,
    /// A range between two values
    Range(Box<RangeType>),
    /// A decimal (floating point) value
    Decimal,
    /// A filesize in bytes
    Bytesize,
    /// A string of text
    String,
    /// A line of text (a string with trailing line ending)
    Line,
    /// A path through a table
    ColumnPath,
    /// A glob pattern (like foo*)
    Pattern,
    /// A boolean value
    Boolean,
    /// A date value (in UTC)
    Date,
    /// A data duration value
    Duration,
    /// A filepath value
    Path,
    /// A binary (non-text) buffer value
    Binary,

    /// A row of data
    Row(Row),
    /// A full table of data
    Table(Vec<Type>),

    /// A block of script (TODO)
    Block,
    /// An error value (TODO)
    Error,

    /// Beginning of stream marker (used as bookend markers rather than actual values)
    BeginningOfStream,
    /// End of stream marker (used as bookend markers rather than actual values)
    EndOfStream,
}

// pub enum PositionalType {
//     /// A mandatory positional argument with the expected shape of the value
//     Mandatory(String, SyntaxShape),
//     /// An optional positional argument with the expected shape of the value
//     Optional(String, SyntaxShape),
// }

// pub struct Signature {
//     /// The name of the command. Used when calling the command
//     pub name: String,
//     /// Usage instructions about the command
//     pub usage: String,
//     /// The list of positional arguments, both required and optional, and their corresponding types and help text
//     pub positional: Vec<(PositionalType, Description)>,
//     /// After the positional arguments, a catch-all for the rest of the arguments that might follow, their type, and help text
//     pub rest_positional: Option<(SyntaxShape, Description)>,
//     /// The named flags with corresponding type and help text
//     pub named: IndexMap<String, (NamedType, Description)>,
//     /// The type of values being sent out from the command into the pipeline, if any
//     pub yields: Option<Type>,
//     /// The type of values being read in from the pipeline into the command, if any
//     pub input: Option<Type>,
//     /// If the command is expected to filter data, or to consume it (as a sink)
//     pub is_filter: bool,
// }

// impl Signature {
//     /// Create a new command signature with the given name
//     pub fn new(name: impl Into<String>) -> Signature {
//         Signature {
//             name: name.into(),
//             usage: String::new(),
//             positional: vec![],
//             rest_positional: None,
//             named: indexmap::indexmap! {"help".into() => (NamedType::Switch(Some('h')), "Display this help message".into())},
//             is_filter: false,
//             yields: None,
//             input: None,
//         }
//     }

//     /// Create a new signature
//     pub fn build(name: impl Into<String>) -> Signature {
//         Signature::new(name.into())
//     }

//     /// Add a description to the signature
//     pub fn desc(mut self, usage: impl Into<String>) -> Signature {
//         self.usage = usage.into();
//         self
//     }

//     /// Add a required positional argument to the signature
//     pub fn required(
//         mut self,
//         name: impl Into<String>,
//         ty: impl Into<SyntaxShape>,
//         desc: impl Into<String>,
//     ) -> Signature {
//         self.positional.push((
//             PositionalType::Mandatory(name.into(), ty.into()),
//             desc.into(),
//         ));

//         self
//     }

//     /// Add an optional positional argument to the signature
//     pub fn optional(
//         mut self,
//         name: impl Into<String>,
//         ty: impl Into<SyntaxShape>,
//         desc: impl Into<String>,
//     ) -> Signature {
//         self.positional.push((
//             PositionalType::Optional(name.into(), ty.into()),
//             desc.into(),
//         ));

//         self
//     }

//     /// Add an optional named flag argument to the signature
//     pub fn named(
//         mut self,
//         name: impl Into<String>,
//         ty: impl Into<SyntaxShape>,
//         desc: impl Into<String>,
//         short: Option<char>,
//     ) -> Signature {
//         let s = short.and_then(|c| {
//             debug_assert!(!self.get_shorts().contains(&c));
//             Some(c)
//         });
//         self.named.insert(
//             name.into(),
//             (NamedType::Optional(s, ty.into()), desc.into()),
//         );

//         self
//     }

//     /// Add a required named flag argument to the signature
//     pub fn required_named(
//         mut self,
//         name: impl Into<String>,
//         ty: impl Into<SyntaxShape>,
//         desc: impl Into<String>,
//         short: Option<char>,
//     ) -> Signature {
//         let s = short.and_then(|c| {
//             debug_assert!(!self.get_shorts().contains(&c));
//             Some(c)
//         });

//         self.named.insert(
//             name.into(),
//             (NamedType::Mandatory(s, ty.into()), desc.into()),
//         );

//         self
//     }

//     /// Add a switch to the signature
//     pub fn switch(
//         mut self,
//         name: impl Into<String>,
//         desc: impl Into<String>,
//         short: Option<char>,
//     ) -> Signature {
//         let s = short.and_then(|c| {
//             debug_assert!(!self.get_shorts().contains(&c));
//             Some(c)
//         });

//         self.named
//             .insert(name.into(), (NamedType::Switch(s), desc.into()));
//         self
//     }

//     /// Set the filter flag for the signature
//     pub fn filter(mut self) -> Signature {
//         self.is_filter = true;
//         self
//     }

//     /// Set the type for the "rest" of the positional arguments
//     pub fn rest(mut self, ty: SyntaxShape, desc: impl Into<String>) -> Signature {
//         self.rest_positional = Some((ty, desc.into()));
//         self
//     }

//     /// Add a type for the output of the command to the signature
//     pub fn yields(mut self, ty: Type) -> Signature {
//         self.yields = Some(ty);
//         self
//     }

//     /// Add a type for the input of the command to the signature
//     pub fn input(mut self, ty: Type) -> Signature {
//         self.input = Some(ty);
//         self
//     }

//     /// Get list of the short-hand flags
//     pub fn get_shorts(&self) -> Vec<char> {
//         let mut shorts = Vec::new();
//         for (_, (t, _)) in &self.named {
//             if let Some(c) = t.get_short() {
//                 shorts.push(c);
//             }
//         }
//         shorts
//     }

//     pub fn has_switch(&self, flag: &str) -> bool {
//         for (name, ty) in &self.named {
//             if name == flag {
//                 match ty {
//                     (NamedType::Switch(_), _) => {
//                         return true;
//                     }
//                     _ => {}
//                 }
//             }
//         }
//         false
//     }
// }

// impl HasSpan for Signature {
//     fn span(&self) -> Span {
//         self.span
//     }
// }

// impl PrettyDebugWithSource for Signature {
//     fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
//         self.unspanned.pretty_debug(source)
//     }
// }

#[derive(Debug, Clone)]
pub struct Signature {
    pub(crate) unspanned: nu_protocol::Signature,
    span: Span,
}

impl Signature {
    pub fn new(unspanned: nu_protocol::Signature, span: impl Into<Span>) -> Signature {
        Signature {
            unspanned,
            span: span.into(),
        }
    }
}

impl HasSpan for Signature {
    fn span(&self) -> Span {
        self.span
    }
}

impl PrettyDebugWithSource for Signature {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        self.unspanned.pretty_debug(source)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExternalArg {
    pub arg: String,
    pub tag: Tag,
}

impl ExternalArg {
    pub fn has(&self, name: &str) -> bool {
        self.arg == name
    }

    pub fn is_it(&self) -> bool {
        self.has("$it")
    }

    pub fn is_nu(&self) -> bool {
        self.has("$nu")
    }

    pub fn looks_like_it(&self) -> bool {
        self.arg.starts_with("$it") && (self.arg.starts_with("$it.") || self.is_it())
    }

    pub fn looks_like_nu(&self) -> bool {
        self.arg.starts_with("$nu") && (self.arg.starts_with("$nu.") || self.is_nu())
    }
}

impl std::ops::Deref for ExternalArg {
    type Target = str;

    fn deref(&self) -> &str {
        &self.arg
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExternalArgs {
    pub list: Vec<ExternalArg>,
    pub span: Span,
}

impl ExternalArgs {
    pub fn iter(&self) -> impl Iterator<Item = &ExternalArg> {
        self.list.iter()
    }
}

impl std::ops::Deref for ExternalArgs {
    type Target = [ExternalArg];

    fn deref(&self) -> &[ExternalArg] {
        &self.list
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExternalCommand {
    pub name: String,

    pub name_tag: Tag,
    pub args: ExternalArgs,
}

impl ExternalCommand {
    pub fn has_it_argument(&self) -> bool {
        self.args.iter().any(|arg| arg.looks_like_it())
    }

    pub fn has_nu_argument(&self) -> bool {
        self.args.iter().any(|arg| arg.looks_like_nu())
    }
}

impl PrettyDebug for ExternalCommand {
    fn pretty(&self) -> DebugDocBuilder {
        b::typed(
            "external command",
            b::description(&self.name)
                + b::preceded(
                    b::space(),
                    b::intersperse(
                        self.args.iter().map(|a| b::primitive(a.arg.to_string())),
                        b::space(),
                    ),
                ),
        )
    }
}

impl HasSpan for ExternalCommand {
    fn span(&self) -> Span {
        self.name_tag.span.until(self.args.span)
    }
}
