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

    pub fn iter(&self) -> impl Iterator<Item = &Type> + Clone {
        self.into_iter()
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

impl<'a> IntoIterator for &'a OneOf {
    type Item = &'a Type;
    type IntoIter = std::slice::Iter<'a, Type>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
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
        Some(match (self.is_empty(), other.is_empty()) {
            (true, true) => TypeRelation::Equal,
            (true, false) => TypeRelation::Subtype,
            (false, true) => TypeRelation::Supertype,
            (false, false) => match (self.is_subtype_of(other), self.is_supertype_of(other)) {
                (true, true) => TypeRelation::Equal,
                (true, false) => TypeRelation::Subtype,
                (false, true) => TypeRelation::Supertype,
                (false, false) => return None,
            },
        })
    }

    fn is_subtype_of(&self, other: &Self) -> bool {
        match (self.is_empty(), other.is_empty()) {
            (true, _) => true,
            (_, true) => false,
            _ => self.iter().all(|ty| ty.is_subtype_of(other)),
        }
    }

    fn is_supertype_of(&self, other: &Self) -> bool {
        other.is_subtype_of(self)
    }

    fn is_assignable_to(&self, dst: &Self) -> bool {
        let dst_tys = dst;
        let src_tys = self;
        match (dst_tys.is_empty(), src_tys.is_empty()) {
            (_, true) => true,
            (true, _) => false,
            _ => src_tys
                .iter()
                .any(|src_ty| dst_tys.iter().any(|dst_ty| src_ty.is_assignable_to(dst_ty))),
        }
    }
}

impl CompareTypes<Type> for OneOf {
    fn compare_types(&self, other: &Type) -> Option<TypeRelation> {
        Some(match other {
            Type::OneOf(other) => return self.compare_types(other),
            Type::Any => TypeRelation::Subtype,
            // `oneof<>` is an uninhibited type, so it's kind of like our bottom type
            _ if self.is_empty() => TypeRelation::Subtype,
            _ => match (self.is_subtype_of(other), self.is_supertype_of(other)) {
                (true, true) => TypeRelation::Equal,
                (true, false) => TypeRelation::Subtype,
                (false, true) => TypeRelation::Supertype,
                (false, false) => return None,
            },
        })
    }

    fn is_subtype_of(&self, other: &Type) -> bool {
        let sub_tys = self;
        let super_ty = other;
        sub_tys.iter().all(|sub_ty| sub_ty.is_subtype_of(super_ty))
    }

    fn is_supertype_of(&self, other: &Type) -> bool {
        let super_tys = self;
        let sub_ty = other;
        super_tys
            .iter()
            .any(|super_ty| super_ty.is_supertype_of(sub_ty))
    }

    fn is_assignable_to(&self, dst: &Type) -> bool {
        let src = self;

        if src.is_empty() {
            true
        } else {
            src.iter().any(|src_ty| src_ty.is_assignable_to(dst))
        }
    }
}

impl CompareTypes<OneOf> for Type {
    fn compare_types(&self, other: &OneOf) -> Option<TypeRelation> {
        Some(
            match (self.is_subtype_of(other), self.is_supertype_of(other)) {
                (true, true) => TypeRelation::Equal,
                (true, false) => TypeRelation::Subtype,
                (false, true) => TypeRelation::Supertype,
                (false, false) => return None,
            },
        )
    }

    fn is_subtype_of(&self, other: &OneOf) -> bool {
        other.is_supertype_of(self)
    }

    fn is_supertype_of(&self, other: &OneOf) -> bool {
        other.is_subtype_of(self)
    }

    fn is_assignable_to(&self, dst: &OneOf) -> bool {
        let src = self;
        dst.iter().any(|dst_ty| src.is_assignable_to(dst_ty))
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
