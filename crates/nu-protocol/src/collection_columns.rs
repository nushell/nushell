use crate::{CompareTypes, TypeRelation, TypeSet};
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
/// # use nu_protocol::{CollectionColumns, Type, TypeSet};
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
///     foo.union(bar),
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

impl<T> CollectionColumns<T>
where
    T: TypeSet + Clone,
{
    fn widen_fields(lhs: Box<[(String, T)]>, rhs: Box<[(String, T)]>) -> Box<[(String, T)]> {
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
            let mut big_map: HashMap<String, T> = big.into_iter().collect();
            small
                .into_iter()
                .filter_map(|(col, typ)| big_map.remove(&col).map(|b_typ| (col, typ.union(b_typ))))
                .collect()
        } else {
            small
                .into_iter()
                .filter_map(|(col, typ)| {
                    big.iter()
                        .find_map(|(b_col, b_typ)| (&col == b_col).then(|| b_typ.clone()))
                        .map(|b_typ| (col, typ.union(b_typ)))
                })
                .collect()
        }
    }
}

impl<T> CompareTypes for CollectionColumns<T>
where
    T: CompareTypes,
{
    fn compare_types(&self, other: &Self) -> Option<TypeRelation> {
        let self_fields = self.fields.as_ref();
        let other_fields = other.fields.as_ref();

        // Handle the simplest cases
        match (self_fields, other_fields) {
            ([], []) => return Some(TypeRelation::Equal),
            ([], _) => return Some(TypeRelation::Supertype),
            (_, []) => return Some(TypeRelation::Subtype),
            _ => (),
        }

        let lhs_super = match self.fields.len().cmp(&other_fields.len()) {
            std::cmp::Ordering::Less => true,
            std::cmp::Ordering::Greater => false,
            std::cmp::Ordering::Equal => {
                // start neutral
                let mut state = TypeRelation::Equal;
                for (name, lhs_ty) in self_fields {
                    let (_, rhs_ty) = other_fields.iter().find(|(o_name, _)| o_name == name)?;
                    if lhs_ty.is_any() || rhs_ty.is_any() {
                        continue;
                    }
                    state = state.combine(lhs_ty.compare_types(rhs_ty)?)?;
                }

                return Some(state);
            }
        };

        let (super_ty, sub_ty) = match lhs_super {
            true => (self_fields, other_fields),
            false => (other_fields, self_fields),
        };

        for (name, super_elem_ty) in super_ty {
            let (_, sub_elem_ty) = sub_ty.iter().find(|(o_name, _)| o_name == name)?;
            match super_elem_ty.compare_types(sub_elem_ty)? {
                TypeRelation::Equal | TypeRelation::Supertype => (),
                TypeRelation::Subtype => return None,
            }
        }

        Some(match lhs_super {
            true => TypeRelation::Supertype,
            false => TypeRelation::Subtype,
        })
    }

    /// Our type system uses the empty record as both the bottom and the top type of records
    fn is_any(&self) -> bool {
        self.fields.is_empty()
    }

    fn is_assignable_to(&self, dst: &Self) -> bool {
        let src = self;
        src.is_any() || dst.is_any() || src.is_subtype_of(dst)
    }
}

impl<T> TypeSet for CollectionColumns<T>
where
    T: TypeSet + Clone,
{
    fn union(self, other: Self) -> Self {
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

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::Type;

    #[test]
    fn equal_size() {
        let param_ty: CollectionColumns<Type> =
            vec![("foo".into(), Type::one_of([Type::Int, Type::Nothing]))].into();
        let arg_ty: CollectionColumns<Type> = vec![("foo".into(), Type::Int)].into();

        assert_eq!(
            param_ty.compare_types(&arg_ty),
            Some(TypeRelation::Supertype)
        );
    }
}
