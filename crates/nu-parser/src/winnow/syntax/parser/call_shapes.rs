//! The parameters a call's arguments are read with: nu's `Signature` as its
//! `parse_internal_call` reads it, each positional's shape, the rest parameter's, and the
//! flags with the shape of the value each takes.
//!
//! The parser reads arguments by them only without a [`super::CommandLookup`] (a
//! [`super::ParseConfig`], whose built-in commands carry their signatures, and the commands
//! the text defines itself): with a lookup the consumer reads each argument by the real
//! signature.

use std::ops::Deref;
use std::rc::Rc;
use std::sync::Arc;

use crate::winnow::syntax::ast::{
    Expr, Expression, Parameter, ParameterKind, Signature, SyntaxShape,
};

/// The [`CallShapes`] of a command: a configured command's, shared by every parse with its
/// [`super::ParseConfig`], or those of a command the text defines.
#[derive(Clone, Debug)]
pub enum CommandShapes<'a> {
    /// A configured command's.
    Configured(Arc<CallShapes<'static>>),
    /// A `def`'s or `extern`'s of the text.
    Defined(Rc<CallShapes<'a>>),
}

impl<'a> Deref for CommandShapes<'a> {
    type Target = CallShapes<'a>;

    fn deref(&self) -> &CallShapes<'a> {
        match self {
            Self::Configured(shapes) => shapes,
            Self::Defined(shapes) => shapes,
        }
    }
}

/// A command's parameters, as a call to it reads its arguments.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CallShapes<'a> {
    /// The positional parameters' shapes: the required ones, then the optional ones (nu's
    /// `get_positional` order).
    positional: Vec<SyntaxShape<'a>>,
    /// The rest parameter's shape.
    rest: Option<SyntaxShape<'a>>,
    /// The flags.
    flags: Vec<FlagShape<'a>>,
}

/// A flag of a [`CallShapes`].
#[derive(Clone, Debug, PartialEq)]
pub struct FlagShape<'a> {
    /// The long name, without dashes.
    long: Option<&'a str>,
    /// The short letter.
    short: Option<char>,
    /// The shape of the value the flag takes; `None` for a switch.
    pub value: Option<SyntaxShape<'a>>,
}

impl<'a> CallShapes<'a> {
    /// The parameters of `signature`. As in nu, a parameter with a default value and no type
    /// takes the default's type (a flag then takes a value), and one with a default is
    /// optional.
    pub fn new(signature: &Signature<'a>) -> Self {
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
                    long: long.map(|long| long.item),
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
    pub fn positional(&self, index: usize) -> Option<&SyntaxShape<'a>> {
        self.positional.get(index).or(self.rest.as_ref())
    }

    /// The flag named `--name`.
    pub fn long_flag(&self, name: &str) -> Option<&FlagShape<'a>> {
        self.flags.iter().find(|flag| flag.long == Some(name))
    }

    /// The flag named `-letter`.
    pub fn short_flag(&self, letter: char) -> Option<&FlagShape<'a>> {
        self.flags.iter().find(|flag| flag.short == Some(letter))
    }
}

/// The shape a parameter is read with: its type's, else its default value's type's, else none
/// (`any` for a positional, a switch for a flag).
fn parameter_shape<'a>(parameter: &Parameter<'a>) -> Option<SyntaxShape<'a>> {
    match (&parameter.ty, &parameter.default) {
        (Some(ty), _) => Some(ty.shape.clone()),
        (None, Some(default)) => Some(default_shape(default)),
        (None, None) => None,
    }
}

/// The shape of a default value's type (nu's `Type::to_shape` of the value's type), for the
/// values whose type their syntax tells; `any` for the rest.
fn default_shape<'a>(default: &Expression<'a>) -> SyntaxShape<'a> {
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
        Expr::List(_) => SyntaxShape::List(None),
        Expr::Table(_) => SyntaxShape::Table(Vec::new()),
        Expr::Record(_) => SyntaxShape::Record(Vec::new()),
        Expr::Closure(_) => SyntaxShape::Closure,
        _ => SyntaxShape::Any,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::winnow::syntax::ParseConfig;
    use crate::winnow::syntax::builtin_commands::BUILTIN_SIGNATURES;

    /// Every generated signature reads, and names only shapes the parser knows: a shape it
    /// read as a type alias's name would read every argument as `any`.
    #[test]
    fn every_builtin_signature_reads() {
        let config = ParseConfig::new();
        let mut unread = Vec::new();
        for (name, text) in BUILTIN_SIGNATURES {
            let Some(shapes) = config.call_shapes(name) else {
                unread.push(format!("{name}: {text}"));
                continue;
            };
            let flag_shapes = shapes.flags.iter().filter_map(|flag| flag.value.as_ref());
            let all = shapes
                .positional
                .iter()
                .chain(&shapes.rest)
                .chain(flag_shapes);
            if all
                .into_iter()
                .any(|shape| matches!(shape, SyntaxShape::TypeAlias(_)))
            {
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
