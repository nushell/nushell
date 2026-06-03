use crate::Type;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[allow(unused_imports)]
use crate::SyntaxShape;

/// Very basic ordered mapping, essentially a list of pairs.
///
/// Handles logic common to [`SyntaxShape::Record`], [`SyntaxShape::Table`], [`Type::Record`],
/// [`Type::Table`], and possibly any other ordered mapping.
///
/// Implements [`Display`] for `T: Display`:
/// ```rust
/// # use nu_protocol::{CollectionColumns, Type};
/// let cols = CollectionColumns::from(vec![
///     ("a".to_string(), 1),
///     ("b".to_string(), 2),
/// ]);
/// assert_eq!(cols.to_string(), "<a: 1, b: 2>");
/// ```
///
/// Type widening (union) for [`Type`]:
/// ```rust
/// # use nu_protocol::{CollectionColumns, Type};
/// let foo = CollectionColumns::from(vec![
///     ("a".to_string(), Type::Int),
///     ("b".to_string(), Type::String),
/// ]);
/// let bar = CollectionColumns::from(vec![
///     ("a".to_string(), Type::Float),
///     ("b".to_string(), Type::Int),
///     ("c".to_string(), Type::Date),
/// ]);
/// assert_eq!(
///     foo.widen(bar),
///     CollectionColumns::from(vec![
///         ("a".to_string(), Type::Number),
///         ("b".to_string(), Type::OneOf([Type::String, Type::Int].into())),
///     ])
/// );
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash, Ord, PartialOrd)]
#[serde(transparent)]
pub struct CollectionColumns<T> {
    pub fields: Box<[(String, T)]>,
}

impl<T> CollectionColumns<T> {
    pub fn map<U>(&self, f: impl Fn(&T) -> U) -> CollectionColumns<U> {
        let Self { fields } = self;
        CollectionColumns {
            fields: fields.into_iter().map(|(k, v)| (k.clone(), f(v))).collect(),
        }
    }
}

impl<T> CollectionColumns<T> {
    pub fn new(fields: Box<[(String, T)]>) -> Self {
        Self { fields }
    }
}

impl<T> FromIterator<(String, T)> for CollectionColumns<T> {
    fn from_iter<I: IntoIterator<Item = (String, T)>>(iter: I) -> Self {
        Self {
            fields: iter.into_iter().collect(),
        }
    }
}

impl<T> From<Vec<(String, T)>> for CollectionColumns<T> {
    fn from(value: Vec<(String, T)>) -> Self {
        Self {
            fields: value.into_boxed_slice(),
        }
    }
}

impl<T> From<Box<[(String, T)]>> for CollectionColumns<T> {
    fn from(value: Box<[(String, T)]>) -> Self {
        Self { fields: value }
    }
}

impl CollectionColumns<Type> {
    pub fn widen(self, other: Self) -> Self {
        let Self {
            fields: self_fields,
        } = self;
        let Self {
            fields: other_fields,
        } = other;

        Self {
            fields: Self::widen_fields(self_fields, other_fields),
        }
    }

    fn widen_fields(
        lhs: Box<[(String, Type)]>,
        rhs: Box<[(String, Type)]>,
    ) -> Box<[(String, Type)]> {
        if lhs.is_empty() || rhs.is_empty() {
            return [].into();
        }

        // iterate the shorter list to reduce quadratic behaviour
        let (small, big) = if lhs.len() <= rhs.len() {
            (lhs, rhs)
        } else {
            (rhs, lhs)
        };

        const MAP_THRESH: usize = 16;
        if big.len() > MAP_THRESH {
            use std::collections::HashMap;
            let mut big_map: HashMap<String, Type> = big.into_iter().collect();
            small
                .into_iter()
                .filter_map(|(col, typ)| big_map.remove(&col).map(|b_typ| (col, typ.widen(b_typ))))
                .collect()
        } else {
            small
                .into_iter()
                .filter_map(|(col, typ)| {
                    big.iter()
                        .find_map(|(b_col, b_typ)| (&col == b_col).then(|| b_typ.clone()))
                        .map(|b_typ| (col, typ.widen(b_typ)))
                })
                .collect()
        }
    }
}

impl<T> Default for CollectionColumns<T> {
    fn default() -> Self {
        Self {
            fields: Default::default(),
        }
    }
}

impl<T> Display for CollectionColumns<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.fields.as_ref() {
            [] => Ok(()),
            [(name, shape), tail @ ..] => {
                write!(f, "<{name}: {shape}")?;
                for (name, shape) in tail {
                    write!(f, ", {name}: {shape}")?;
                }

                write!(f, ">")?;
                Ok(())
            }
        }
    }
}
