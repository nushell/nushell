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
///         ("b".to_string(), Type::one_of([Type::String, Type::Int])),
///     ])
/// );
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash, Ord, PartialOrd)]
#[serde(transparent)]
pub struct CollectionColumns<T> {
    fields: Box<[(String, T)]>,
}

impl<T> CollectionColumns<T> {
    pub fn get<'s>(&'s self, key: &'_ str) -> Option<&'s T> {
        self.iter()
            .find(|(name, _)| name == key)
            .map(|(_, val)| val)
    }

    pub fn map<U>(&self, f: impl Fn(&T) -> U) -> CollectionColumns<U> {
        self.iter().map(|(k, v)| (k.clone(), f(v))).collect()
    }

    pub fn iter(&self) -> impl Iterator<Item = &(String, T)> {
        self.into_iter()
    }

    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    pub fn len(&self) -> usize {
        self.fields.len()
    }
}

impl<T> IntoIterator for CollectionColumns<T> {
    type Item = (String, T);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.fields.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a CollectionColumns<T> {
    type Item = &'a (String, T);
    type IntoIter = std::slice::Iter<'a, (String, T)>;

    fn into_iter(self) -> Self::IntoIter {
        self.fields.iter()
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

impl<'a, T, const N: usize> From<[(&'a str, T); N]> for CollectionColumns<T> {
    fn from(value: [(&'a str, T); N]) -> Self {
        value
            .into_iter()
            .map(|(k, v)| (String::from(k), v))
            .collect()
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

fn element_comparison_helper<T, F, O>(
    lhs: &CollectionColumns<T>,
    rhs: &CollectionColumns<T>,
    f: F,
) -> impl Iterator<Item = Option<O>>
where
    T: CompareTypes,
    F: Fn(&T, &T) -> Option<O>,
{
    lhs.iter()
        .map(move |(lhs_key, lhs_ty)| match rhs.get(lhs_key) {
            Some(rhs_ty) => f(lhs_ty, rhs_ty),
            // if `lhs` has a field `rhs` doesn't despite having at most the same number of
            // columns as `rhs` (see NOTE[1]) then the sets of their keys are disjoint. they
            // can't have a subtyping relation
            None => None,
        })
}

impl<T> CompareTypes for CollectionColumns<T>
where
    T: CompareTypes,
{
    fn compare_types(&self, other: &Self) -> Option<TypeRelation> {
        // for structural subtyping, each field in a type is a "requirement". less
        // fields in the type => less requirements => is supertype of more types
        // e.g.: `{a: any, b: any}` is a supertype of `{a: any, b: any, c: any}`
        //
        // for `self` to be a subtype of `other`:
        // - `self` must have all fields required by `other`. extra fields in `self` are irrelevant
        // - for field `a` in `other`, `self.a` must be a subtype of `other.a`

        match (self.is_empty(), other.is_empty()) {
            (true, true) => return Some(TypeRelation::Equal),
            (true, false) => return Some(TypeRelation::Supertype),
            (false, true) => return Some(TypeRelation::Subtype),
            (false, false) => (),
        }

        // NOTE[1]: with regards to number of columns `lhs` <= `rhs`
        let (flipped, eq, (lhs, rhs)) = match self.fields.len().cmp(&other.fields.len()) {
            std::cmp::Ordering::Less => (false, false, (self, other)),
            std::cmp::Ordering::Equal => (false, true, (self, other)),
            std::cmp::Ordering::Greater => (true, false, (other, self)),
        };

        let start = match eq {
            true => TypeRelation::Equal,
            false => TypeRelation::Supertype,
        };

        let out = element_comparison_helper(lhs, rhs, |lhs_ty, rhs_ty| {
            if lhs_ty.is_any() || rhs_ty.is_any() {
                // Not really" equal", just used to continue without affecting the outcome.
                Some(TypeRelation::Equal)
            } else {
                lhs_ty.compare_types(rhs_ty)
            }
        })
        .try_fold(start, |acc, e| acc.combine(e?))?;

        Some(match flipped {
            true => out.reverse(),
            false => out,
        })
    }

    /// Our type system uses the empty record as both the bottom and the top type of records
    fn is_any(&self) -> bool {
        self.fields.is_empty()
    }

    fn is_assignable_to(&self, dst: &Self) -> bool {
        let src = self;

        (src.is_any() || dst.is_any())
            || element_comparison_helper(dst, src, |dst_ty, src_ty| {
                Some(src_ty.is_assignable_to(dst_ty))
            })
            .try_fold(true, |acc, e| Some(acc && (e?)))
            .unwrap_or(false)
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
    use rstest::rstest;

    use super::*;
    use crate::Type;

    #[rstest]
    #[case(Some(TypeRelation::Equal), [], [])]
    #[case(Some(TypeRelation::Equal),
        [("a", Type::Int)],
        [("a", Type::Int)],
    )]
    #[case(None,
        [("a", Type::Int)],
        [("b", Type::Int)],
    )]
    #[case(Some(TypeRelation::Supertype),
        [("a", Type::Int), ("b", Type::Int)],
        [("a", Type::Int), ("b", Type::Int), ("c", Type::Int)],
    )]
    #[case(None,
        [("name", Type::String), ("attrs", Type::list(Type::Any)), ("desc", Type::String)],
        [("attrs", Type::list(Type::String)), ("desc", Type::String)],
    )]
    fn relations(
        #[case] expected: Option<TypeRelation>,
        #[case] lhs: impl IntoIterator<Item = (&'static str, Type)>,
        #[case] rhs: impl IntoIterator<Item = (&'static str, Type)>,
    ) {
        let lhs = lhs
            .into_iter()
            .map(|(k, ty)| (k.to_owned(), ty))
            .collect::<CollectionColumns<Type>>();
        let rhs = rhs
            .into_iter()
            .map(|(k, ty)| (k.to_owned(), ty))
            .collect::<CollectionColumns<Type>>();

        assert_eq!(lhs.compare_types(&rhs), expected);
        assert_eq!(rhs.compare_types(&lhs), expected.map(TypeRelation::reverse));
    }

    #[rstest]
    #[case(true,
        [("name", Type::String), ("attrs", Type::list(Type::Any)), ("desc", Type::String)],
        [("attrs", Type::list(Type::String)), ("desc", Type::String)],
    )]
    fn is_assignable_to(
        #[case] expected: bool,
        #[case] src: impl IntoIterator<Item = (&'static str, Type)>,
        #[case] dst: impl IntoIterator<Item = (&'static str, Type)>,
    ) {
        let src = src
            .into_iter()
            .map(|(k, ty)| (k.to_owned(), ty))
            .collect::<CollectionColumns<Type>>();
        let dst = dst
            .into_iter()
            .map(|(k, ty)| (k.to_owned(), ty))
            .collect::<CollectionColumns<Type>>();

        assert_eq!(src.is_assignable_to(&dst), expected)
    }
}
