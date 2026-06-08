use std::fmt::Display;

use crate::{CompareTypes, Type, TypeRelation, TypeSet};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Hash, Ord, PartialOrd)]
#[serde(transparent)]
pub struct OneOf {
    items: Box<[Type]>,
}

impl OneOf {
    pub fn add_ty(self, ty: Type) -> Self {
        let OneOf { items } = self;
        let mut items = items.into_vec();
        Self::add_ty_inner(&mut items, ty);
        Self {
            items: items.into(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Type> {
        self.items.iter()
    }

    fn add_ty_inner(this: &mut Vec<Type>, mut ty: Type) {
        // handle nested unions first
        if let Type::OneOf(inner) = ty {
            for sub_t in inner.items {
                Self::add_ty_inner(this, sub_t);
            }
            return;
        }

        for this_ty in this.iter_mut() {
            let one = std::mem::replace(this_ty, Type::Any);
            match Type::flat_widen(one, ty) {
                Ok(new_wide) => {
                    *this_ty = new_wide;
                    return;
                }
                Err((old_one, old_ty)) => {
                    *this_ty = old_one;
                    ty = old_ty;
                }
            }
        }

        this.push(ty);
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl IntoIterator for OneOf {
    type Item = Type;
    type IntoIter = std::vec::IntoIter<Type>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

impl FromIterator<Type> for OneOf {
    fn from_iter<I: IntoIterator<Item = Type>>(iter: I) -> Self {
        let mut vec = Vec::new();
        for ty in iter {
            Self::add_ty_inner(&mut vec, ty);
        }
        Self { items: vec.into() }
    }
}

impl TypeSet for OneOf {
    fn union(self, other: Self) -> Self {
        let OneOf { items: ts } = self;
        let OneOf { items: us } = other;
        let (big, small) = match ts.len() >= us.len() {
            true => (ts, us),
            false => (us, ts),
        };
        let mut out = big.into_vec();
        for t in small {
            Self::add_ty_inner(&mut out, t);
        }
        Self { items: out.into() }
    }
}

impl CompareTypes for OneOf {
    fn compare_types(&self, other: &Self) -> Option<TypeRelation> {
        let self_items = self.items.as_ref();
        let other_items = other.items.as_ref();

        // Handle the simplest cases
        match (self_items, other_items) {
            ([], []) => return Some(TypeRelation::Equal),
            ([], _) => return Some(TypeRelation::Subtype),
            (_, []) => return Some(TypeRelation::Supertype),
            _ => (),
        }

        // iterate the shorter list to reduce quadratic behaviour
        let ((small, big), flipped) = if self_items.len() <= other_items.len() {
            ((self_items, other_items), false)
        } else {
            ((other_items, self_items), true)
        };

        for s_ty in small {
            let _ = big.iter().find(|b_ty| {
                matches!(
                    s_ty.compare_types(*b_ty),
                    Some(TypeRelation::Subtype | TypeRelation::Equal)
                )
            })?;
        }

        Some(match flipped {
            false => TypeRelation::Subtype,
            true => TypeRelation::Supertype,
        })
    }
}

impl CompareTypes<Type> for OneOf {
    fn compare_types(&self, other: &Type) -> Option<TypeRelation> {
        match other {
            // `oneof<>` is an uninhibited type, so it's kind of like our bottom type
            _ if self.is_empty() => Some(TypeRelation::Subtype),
            Type::Any => Some(TypeRelation::Subtype),
            Type::OneOf(other) => self.compare_types(other),
            _ => self
                .items
                .iter()
                .any(|self_ty| {
                    matches!(
                        self_ty.compare_types(other),
                        Some(TypeRelation::Supertype | TypeRelation::Equal)
                    )
                })
                .then_some(TypeRelation::Supertype),
        }
    }
}

impl CompareTypes<OneOf> for Type {
    fn compare_types(&self, other: &OneOf) -> Option<TypeRelation> {
        other.compare_types(self).map(TypeRelation::reverse)
    }
}

impl Display for OneOf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let types = self.items.as_ref();
        write!(f, "oneof")?;
        let [first, rest @ ..] = types else {
            return Ok(());
        };
        write!(f, "<{first}")?;
        for t in rest {
            write!(f, ", {t}")?;
        }
        f.write_str(">")
    }
}
