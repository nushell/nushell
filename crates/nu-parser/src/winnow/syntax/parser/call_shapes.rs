//! The parameters a call's arguments are read with: nu's `Signature` as its
//! `parse_internal_call` reads it, each positional's shape, the rest parameter's, and the
//! flags with the shape of the value each takes. The shapes are nu-protocol's, as an engine's
//! signatures carry them; a signature the parser reads from text has its types converted
//! ([`nu_shape`]).
//!
//! Without a [`super::CommandLookup`] the parser reads arguments by the signatures of a
//! [`super::ParseConfig`]'s built-in commands and of the commands the text defines itself. With
//! one, by the signatures the lookup gives ([`super::CommandLookup::call_shapes`]): an engine's,
//! whatever defined the command.

use std::ops::Deref;
use std::rc::Rc;
use std::sync::Arc;

use nu_protocol::SyntaxShape;

use crate::winnow::syntax::ast::{self, Expr, Expression, Parameter, ParameterKind, Signature};

/// The [`CallShapes`] of a command: a configured command's or a lookup's, shared by every parse
/// that asks for them, or those of a command the text defines.
#[derive(Clone, Debug)]
pub enum CommandShapes {
    /// A configured command's, or one a lookup gave.
    Shared(Arc<CallShapes>),
    /// A `def`'s or `extern`'s of the text.
    Defined(Rc<CallShapes>),
}

impl Deref for CommandShapes {
    type Target = CallShapes;

    fn deref(&self) -> &CallShapes {
        match self {
            Self::Shared(shapes) => shapes,
            Self::Defined(shapes) => shapes,
        }
    }
}

/// A command's parameters, as a call to it reads its arguments.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CallShapes {
    /// The positional parameters' shapes: the required ones, then the optional ones (nu's
    /// `get_positional` order).
    positional: Vec<SyntaxShape>,
    /// The rest parameter's shape.
    rest: Option<SyntaxShape>,
    /// The flags.
    flags: Vec<FlagShape>,
}

/// A flag of a [`CallShapes`].
#[derive(Clone, Debug, PartialEq)]
pub struct FlagShape {
    /// The long name, without dashes.
    long: Option<Box<str>>,
    /// The short letter.
    short: Option<char>,
    /// The shape of the value the flag takes; `None` for a switch.
    pub value: Option<SyntaxShape>,
}

impl CallShapes {
    /// The parameters of `signature`. As in nu, a parameter with a default value and no type
    /// takes the default's type (a flag then takes a value), and one with a default is
    /// optional.
    pub fn new(signature: &Signature<'_>) -> Self {
        let mut required = Vec::new();
        let mut optional = Vec::new();
        let mut rest = None;
        let mut flags = Vec::new();
        for parameter in &signature.params {
            let shape = parameter_shape(parameter);
            match &parameter.kind {
                ParameterKind::Required if parameter.default.is_none() => {
                    required.push(shape.unwrap_or(SyntaxShape::Any));
                }
                ParameterKind::Required | ParameterKind::Optional => {
                    optional.push(shape.unwrap_or(SyntaxShape::Any));
                }
                ParameterKind::Rest => rest = Some(shape.unwrap_or(SyntaxShape::Any)),
                ParameterKind::Flag { long, short } => flags.push(FlagShape {
                    long: long.map(|long| Box::from(long.item)),
                    short: short.map(|short| short.item),
                    value: shape,
                }),
            }
        }
        required.append(&mut optional);
        Self {
            positional: required,
            rest,
            flags,
        }
    }

    /// The parameters of an engine's `signature`, if the parser reads a value of each of their
    /// shapes from one item ([`reads_in_one_item`]). A command that has another (a keyword's
    /// `block`, a row condition, a plugin's custom type) is read without its signature.
    pub fn from_signature(signature: &nu_protocol::Signature) -> Option<Self> {
        let positional: Vec<SyntaxShape> = signature
            .required_positional
            .iter()
            .chain(&signature.optional_positional)
            .map(|parameter| parameter.shape.clone())
            .collect();
        let rest = signature
            .rest_positional
            .as_ref()
            .map(|parameter| parameter.shape.clone());
        let flags: Vec<FlagShape> = signature
            .named
            .iter()
            .map(|flag| FlagShape {
                long: flag.long_name().map(Box::from),
                short: flag.short,
                value: flag.arg.clone(),
            })
            .collect();
        let flag_values = flags.iter().filter_map(|flag| flag.value.as_ref());
        positional
            .iter()
            .chain(&rest)
            .chain(flag_values)
            .all(reads_in_one_item)
            .then_some(Self {
                positional,
                rest,
                flags,
            })
    }

    /// The parameters left for a call through an alias whose target gives the first `given`
    /// positional arguments (`alias ll = ls a`).
    pub fn past_positionals(&self, given: usize) -> Self {
        let mut shapes = self.clone();
        shapes
            .positional
            .drain(..given.min(shapes.positional.len()));
        shapes
    }

    /// The shape of the positional argument at `index` (nu's `get_positional`): a positional
    /// parameter's, the rest parameter's past them, `None` past both.
    pub fn positional(&self, index: usize) -> Option<&SyntaxShape> {
        self.positional.get(index).or(self.rest.as_ref())
    }

    /// The flag named `--name`.
    pub fn long_flag(&self, name: &str) -> Option<&FlagShape> {
        self.flags
            .iter()
            .find(|flag| flag.long.as_deref() == Some(name))
    }

    /// The flag named `-letter`.
    pub fn short_flag(&self, letter: char) -> Option<&FlagShape> {
        self.flags.iter().find(|flag| flag.short == Some(letter))
    }
}

/// The shape a parameter is read with: its type's, else its default value's type's, else none
/// (`any` for a positional, a switch for a flag).
fn parameter_shape(parameter: &Parameter<'_>) -> Option<SyntaxShape> {
    match (&parameter.ty, &parameter.default) {
        (Some(ty), _) => Some(nu_shape(&ty.shape)),
        (None, Some(default)) => Some(default_shape(default)),
        (None, None) => None,
    }
}

/// The shape of a default value's type (nu's `Type::to_shape` of the value's type), for the
/// values whose type their syntax tells; `any` for the rest.
fn default_shape(default: &Expression<'_>) -> SyntaxShape {
    match &default.expr {
        Expr::Bool(_) => SyntaxShape::Boolean,
        Expr::Nothing => SyntaxShape::Nothing,
        Expr::Int(_) => SyntaxShape::Int,
        Expr::Float(_) => SyntaxShape::Float,
        Expr::String(_) | Expr::StringInterpolation(_) => SyntaxShape::String,
        Expr::Binary(_) => SyntaxShape::Binary,
        Expr::Duration(_) => SyntaxShape::Duration,
        Expr::Filesize(_) => SyntaxShape::Filesize,
        Expr::DateTime(_) => SyntaxShape::DateTime,
        Expr::Range(_) => SyntaxShape::Range,
        Expr::CellPath(_) => SyntaxShape::CellPath,
        Expr::List(_) => SyntaxShape::List(Box::new(SyntaxShape::Any)),
        Expr::Table(_) => SyntaxShape::Table(Default::default()),
        Expr::Record(_) => SyntaxShape::Record(Default::default()),
        Expr::Closure(_) => SyntaxShape::Closure(None),
        _ => SyntaxShape::Any,
    }
}

/// Whether the parser reads a value of `shape` from one item: the shapes a type annotation
/// names, which are those of every non-keyword built-in command's parameters.
fn reads_in_one_item(shape: &SyntaxShape) -> bool {
    match shape {
        SyntaxShape::Any
        | SyntaxShape::Binary
        | SyntaxShape::Boolean
        | SyntaxShape::CellPath
        | SyntaxShape::Closure(_)
        | SyntaxShape::DateTime
        | SyntaxShape::Directory
        | SyntaxShape::Duration
        | SyntaxShape::Error
        | SyntaxShape::ExternalArgument
        | SyntaxShape::Float
        | SyntaxShape::Filesize
        | SyntaxShape::GlobPattern
        | SyntaxShape::Int
        | SyntaxShape::Nothing
        | SyntaxShape::Number
        | SyntaxShape::Filepath
        | SyntaxShape::Range
        | SyntaxShape::String => true,
        SyntaxShape::List(element) => reads_in_one_item(element),
        SyntaxShape::Record(columns) | SyntaxShape::Table(columns) => {
            columns.iter().all(|(_, shape)| reads_in_one_item(shape))
        }
        SyntaxShape::OneOf(shapes) => shapes.iter().all(reads_in_one_item),
        _ => false,
    }
}

/// The nu-protocol shape a type annotation names, as nu's `parse_shape_name` reads it. A type
/// alias is `any`: what it names is the consumer's to look up (nu's `expand_type_alias`), and
/// the parser reads a value of it as it reads one of `any`.
pub(crate) fn nu_shape(shape: &ast::SyntaxShape<'_>) -> SyntaxShape {
    let columns = |fields: &[ast::TypeField<'_>]| {
        fields
            .iter()
            .map(|field| (field.name.item.to_string(), nu_shape(&field.ty.shape)))
            .collect()
    };
    match shape {
        ast::SyntaxShape::Any | ast::SyntaxShape::TypeAlias(_) => SyntaxShape::Any,
        ast::SyntaxShape::Binary => SyntaxShape::Binary,
        ast::SyntaxShape::Boolean => SyntaxShape::Boolean,
        ast::SyntaxShape::CellPath => SyntaxShape::CellPath,
        ast::SyntaxShape::Closure => SyntaxShape::Closure(None),
        ast::SyntaxShape::DateTime => SyntaxShape::DateTime,
        ast::SyntaxShape::Directory => SyntaxShape::Directory,
        ast::SyntaxShape::Duration => SyntaxShape::Duration,
        ast::SyntaxShape::Error => SyntaxShape::Error,
        ast::SyntaxShape::ExternalArgument => SyntaxShape::ExternalArgument,
        ast::SyntaxShape::Float => SyntaxShape::Float,
        ast::SyntaxShape::Filesize => SyntaxShape::Filesize,
        ast::SyntaxShape::GlobPattern => SyntaxShape::GlobPattern,
        ast::SyntaxShape::Int => SyntaxShape::Int,
        ast::SyntaxShape::Nothing => SyntaxShape::Nothing,
        ast::SyntaxShape::Number => SyntaxShape::Number,
        ast::SyntaxShape::Filepath => SyntaxShape::Filepath,
        ast::SyntaxShape::Range => SyntaxShape::Range,
        ast::SyntaxShape::String => SyntaxShape::String,
        ast::SyntaxShape::List(element) => SyntaxShape::List(Box::new(
            element
                .as_ref()
                .map_or(SyntaxShape::Any, |element| nu_shape(&element.shape)),
        )),
        ast::SyntaxShape::Record(fields) => SyntaxShape::Record(columns(fields)),
        ast::SyntaxShape::Table(fields) => SyntaxShape::Table(columns(fields)),
        ast::SyntaxShape::OneOf(alternatives) => SyntaxShape::OneOf(
            alternatives
                .iter()
                .map(|alternative| nu_shape(&alternative.shape))
                .collect(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::super::{parse_signatures::parse_signature, working_set::WorkingSet};
    use super::*;
    use crate::winnow::syntax::builtin_commands::BUILTIN_SIGNATURES;
    use crate::winnow::syntax::{ParseConfig, Span};

    /// Every generated signature reads, and names only shapes the parser knows: a shape it
    /// read as a type alias's name would read every argument as `any`. Checked on the signature
    /// as read, before its types become nu-protocol's, where such a name is `any`.
    #[test]
    fn every_builtin_signature_reads() {
        let mut unread = Vec::new();
        for (name, text) in BUILTIN_SIGNATURES {
            let working_set = WorkingSet::new(text, &ParseConfig::empty());
            let reads = parse_signature(&working_set, Span::new(0, text.len()), true).is_ok_and(
                |signature| {
                    !signature.params.iter().any(|parameter| {
                        matches!(
                            parameter.ty.as_ref().map(|ty| &ty.shape),
                            Some(ast::SyntaxShape::TypeAlias(_))
                        )
                    })
                },
            );
            if !reads {
                unread.push(format!("{name}: {text}"));
            }
        }
        assert!(
            unread.is_empty(),
            "signatures that do not read: {unread:#?}"
        );
    }

    /// A built-in's parameters, as a call reads them.
    #[test]
    fn builtin_parameters() {
        let config = ParseConfig::new();
        let Some(trim) = config.call_shapes("str trim") else {
            panic!("`str trim` has a signature");
        };
        assert_eq!(trim.positional(0), Some(&SyntaxShape::CellPath));
        assert_eq!(trim.positional(5), Some(&SyntaxShape::CellPath));
        assert_eq!(
            trim.long_flag("char").and_then(|flag| flag.value.as_ref()),
            Some(&SyntaxShape::String)
        );
        assert_eq!(trim.short_flag('l').map(|flag| &flag.value), Some(&None));
        assert!(
            config.call_shapes("if").is_none(),
            "a keyword has its own parser"
        );
    }
}
