use crate::{
    CollectionColumns, CompareTypes, OneOf, SyntaxShape, TypeRelation, TypeSet, ast::PathMember,
};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, fmt::Display};
#[cfg(test)]
use strum_macros::EnumIter;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Hash, Ord, PartialOrd)]
#[cfg_attr(test, derive(EnumIter))]
pub enum Type {
    /// Top type, supertype of all types
    Any,
    Binary,
    Block,
    Bool,
    CellPath,
    Closure,
    Custom(Box<str>),
    Date,
    Duration,
    Error,
    Filesize,
    Float,
    Int,
    List(Box<Type>),
    #[default]
    Nothing,
    /// Supertype of Int and Float. Equivalent to `oneof<int, float>`
    Number,
    /// Supertype of all types it contains.
    OneOf(OneOf),
    Range,
    Record(CollectionColumns<Type>),
    String,
    Glob,
    Table(CollectionColumns<Type>),
}

fn follow_cell_path_recursive<'a>(
    current: Cow<'a, Type>,
    path_members: &mut dyn Iterator<Item = &'a PathMember>,
) -> Option<Cow<'a, Type>> {
    let Some(first) = path_members.next() else {
        return Some(current);
    };
    match (current.as_ref(), first) {
        (Type::Record(_), PathMember::String { val, .. }) => {
            let next = match current {
                Cow::Borrowed(Type::Record(f)) => {
                    Cow::Borrowed(&f.iter().find(|(name, _)| name == val)?.1)
                }
                Cow::Owned(Type::Record(f)) => {
                    Cow::Owned(f.into_iter().find(|(name, _)| name == val)?.1)
                }
                _ => unreachable!(),
            };
            follow_cell_path_recursive(next, path_members)
        }

        // Table to Record (Int)
        (Type::Table(f), PathMember::Int { .. }) => {
            follow_cell_path_recursive(Cow::Owned(Type::Record(f.clone())), path_members)
        }

        // Table to List (String)
        (Type::Table(columns), PathMember::String { val, .. }) => {
            let (_, sub_type) = columns.iter().find(|(name, _)| name == val)?;
            let list_type = Type::List(Box::new(sub_type.clone()));
            follow_cell_path_recursive(Cow::Owned(list_type), path_members)
        }

        (Type::List(_), PathMember::Int { .. }) => {
            let next = match current {
                Cow::Borrowed(Type::List(i)) => Cow::Borrowed(i.as_ref()),
                Cow::Owned(Type::List(i)) => Cow::Owned(*i),
                _ => unreachable!(),
            };
            follow_cell_path_recursive(next, path_members)
        }

        // List of Records indexed by key names
        (Type::List(_), PathMember::String { .. }) => {
            let next = match current {
                Cow::Borrowed(Type::List(i)) => Cow::Borrowed(i.as_ref()),
                Cow::Owned(Type::List(i)) => Cow::Owned(*i),
                _ => unreachable!(),
            };

            let mut found_int_member = false;
            let mut new_iter = std::iter::once(first).chain(path_members).filter(|pm| {
                let first_int = !found_int_member && matches!(pm, PathMember::Int { .. });
                if first_int {
                    found_int_member = true;
                }
                !first_int
            });
            let inner_ty = follow_cell_path_recursive(next, &mut new_iter);

            // If there's no int path member, need to wrap in a List type
            // e.g. [{foo: bar}].foo -> [bar], list<record<foo: string>> -> list<string>
            if found_int_member {
                inner_ty
            } else {
                inner_ty.map(|inner_ty| Cow::Owned(Type::List(Box::new(inner_ty.into_owned()))))
            }
        }

        _ => None,
    }
}

impl Type {
    pub fn list(inner: Type) -> Self {
        Self::List(Box::new(inner))
    }

    /// Creates a OneOf type from an iterator of types.
    /// Flattens any nested OneOf types and removes duplicates.
    pub fn one_of(types: impl IntoIterator<Item = Type>) -> Self {
        Self::OneOf(OneOf::from_iter(types))
    }

    pub fn record() -> Self {
        Self::Record(Default::default())
    }

    pub fn table() -> Self {
        Self::Table(Default::default())
    }

    pub fn custom(name: impl Into<Box<str>>) -> Self {
        Self::Custom(name.into())
    }

    /// Returns supertype of arguments without creating a `oneof`, or falling back to `any` (unless one or both of the arguments are `any`)
    pub(crate) fn flat_widen(lhs: Type, rhs: Type) -> Result<Type, (Type, Type)> {
        match (lhs, rhs) {
            // short circuit on `any`
            (Type::Any, _) | (_, Type::Any) => Ok(Type::Any),

            // primitive number hierarchy is extremely common
            (Type::Int, Type::Float) | (Type::Float, Type::Int) => Ok(Type::Number),

            // despite their subtyping relation, these pairs should not combine into one or the other
            tys @ ((Type::Glob, Type::String)
            | (Type::String, Type::Glob)
            | (Type::String | Type::Int, Type::CellPath)
            | (Type::CellPath, Type::String | Type::Int)) => Err(tys),

            // widen structural collections without checking for subtyping
            (Type::Record(lhs), Type::Record(rhs)) => Ok(Type::Record(lhs.union(rhs))),
            (Type::Table(lhs), Type::Table(rhs)) => Ok(Type::Table(lhs.union(rhs))),

            // We want to have `oneof<list<T>, table>`, regardless whether one counts as a subtype
            // of the other.
            tys @ ((Type::List(_), Type::Table(_)) | (Type::Table(_), Type::List(_))) => Err(tys),

            // If one type is already a subtype of the other, we can skip all of the heavier logic below.
            (lhs, rhs) => match lhs.compare_types(&rhs) {
                Some(rel) => Ok(match rel {
                    TypeRelation::Subtype => rhs,
                    TypeRelation::Equal => lhs,
                    TypeRelation::Supertype => lhs,
                }),
                // Fallback - the two types are unrelated. Move them out so that callers don't have to clone again.
                None => Err((lhs, rhs)),
            },
        }
    }

    /// Returns a supertype of all types within `it` *that is not `Any`*.
    /// If `it` contains `Type::Any`, short circuits and returns `None`.
    pub fn supertype_of(it: impl IntoIterator<Item = Type>) -> Option<Self> {
        let mut it = it.into_iter();
        it.next().and_then(|head| {
            it.try_fold(head, |acc, e| match acc.union(e) {
                Type::Any => None,
                r => Some(r),
            })
        })
    }

    pub fn is_numeric(&self) -> bool {
        matches!(self, Type::Int | Type::Float | Type::Number)
    }

    pub fn is_list(&self) -> bool {
        matches!(self, Type::List(_))
    }

    /// Does this type represent a data structure containing values that can be addressed using 'cell paths'?
    pub fn accepts_cell_paths(&self) -> bool {
        matches!(self, Type::List(_) | Type::Record(_) | Type::Table(_))
    }

    pub fn to_shape(&self) -> SyntaxShape {
        match self {
            Type::Int => SyntaxShape::Int,
            Type::Float => SyntaxShape::Float,
            Type::Range => SyntaxShape::Range,
            Type::Bool => SyntaxShape::Boolean,
            Type::String => SyntaxShape::String,
            Type::Block => SyntaxShape::Block, // FIXME needs more accuracy
            Type::Closure => SyntaxShape::Closure(None), // FIXME needs more accuracy
            Type::CellPath => SyntaxShape::CellPath,
            Type::Duration => SyntaxShape::Duration,
            Type::Date => SyntaxShape::DateTime,
            Type::Filesize => SyntaxShape::Filesize,
            Type::List(x) => SyntaxShape::List(Box::new(x.to_shape())),
            Type::Number => SyntaxShape::Number,
            Type::OneOf(types) => SyntaxShape::OneOf(types.iter().map(Type::to_shape).collect()),
            Type::Nothing => SyntaxShape::Nothing,
            Type::Record(entries) => SyntaxShape::Record(entries.map(Type::to_shape)),
            Type::Table(columns) => SyntaxShape::Table(columns.map(Type::to_shape)),
            Type::Any => SyntaxShape::Any,
            Type::Error => SyntaxShape::Any,
            Type::Binary => SyntaxShape::Binary,
            Type::Custom(_) => SyntaxShape::Any,
            Type::Glob => SyntaxShape::GlobPattern,
        }
    }

    /// Get a string representation, without inner type specification of lists,
    /// tables and records (get `list` instead of `list<any>`
    pub fn get_non_specified_string(&self) -> String {
        match self {
            Type::Closure => String::from("closure"),
            Type::Bool => String::from("bool"),
            Type::Block => String::from("block"),
            Type::CellPath => String::from("cell-path"),
            Type::Date => String::from("datetime"),
            Type::Duration => String::from("duration"),
            Type::Filesize => String::from("filesize"),
            Type::Float => String::from("float"),
            Type::Int => String::from("int"),
            Type::Range => String::from("range"),
            Type::Record(_) => String::from("record"),
            Type::Table(_) => String::from("table"),
            Type::List(_) => String::from("list"),
            Type::Nothing => String::from("nothing"),
            Type::Number => String::from("number"),
            Type::OneOf(_) => String::from("oneof"),
            Type::String => String::from("string"),
            Type::Any => String::from("any"),
            Type::Error => String::from("error"),
            Type::Binary => String::from("binary"),
            Type::Custom(_) => String::from("custom"),
            Type::Glob => String::from("glob"),
        }
    }

    pub fn follow_cell_path<'a>(&'a self, path_members: &'a [PathMember]) -> Option<Cow<'a, Self>> {
        follow_cell_path_recursive(Cow::Borrowed(self), &mut path_members.iter())
    }
}

impl CompareTypes for Type {
    fn compare_types(&self, other: &Self) -> Option<TypeRelation> {
        match (self, other) {
            (_, Type::Any) => Some(TypeRelation::Subtype),
            (Type::Any, _) => Some(TypeRelation::Supertype),

            // I don't know how this was decided but this is the behavior that was present in the
            // parser
            (Type::Closure, Type::Block) => Some(TypeRelation::Supertype),
            (Type::Block, Type::Closure) => Some(TypeRelation::Subtype),

            // We want `get`/`select`/etc to accept string and int values, so it's convenient to
            // use them with variables, without having to explicitly convert them into cell-paths
            (Type::String | Type::Int, Type::CellPath) => Some(TypeRelation::Subtype),
            (Type::CellPath, Type::String | Type::Int) => Some(TypeRelation::Supertype),

            (Type::Float | Type::Int, Type::Number) => Some(TypeRelation::Subtype),
            (Type::Number, Type::Float | Type::Int) => Some(TypeRelation::Supertype),

            (Type::Glob, Type::String) => Some(TypeRelation::Supertype),
            (Type::String, Type::Glob) => Some(TypeRelation::Subtype),

            // List is covariant
            (Type::List(t), Type::List(u)) => t.compare_types(u.as_ref()),

            (Type::Record(this), Type::Record(that)) | (Type::Table(this), Type::Table(that)) => {
                this.compare_types(that)
            }

            (Type::Table(table_cols), Type::List(list_elem)) => match list_elem.as_ref() {
                Type::Any => Some(TypeRelation::Subtype),
                Type::Record(record_cols) => table_cols.compare_types(record_cols),
                _ => None,
            },
            (Type::List(list_elem), Type::Table(table_cols)) => match list_elem.as_ref() {
                Type::Any => Some(TypeRelation::Supertype),
                Type::Record(record_cols) => record_cols.compare_types(table_cols),
                _ => None,
            },

            (Type::OneOf(lhs_oneof), Type::OneOf(rhs_oneof)) => lhs_oneof.compare_types(rhs_oneof),
            (Type::OneOf(lhs_oneof), rhs) => lhs_oneof.compare_types(rhs),
            (lhs, Type::OneOf(rhs_oneof)) => lhs.compare_types(rhs_oneof),

            (t, u) if t == u => Some(TypeRelation::Equal),

            _ => None,
        }
    }

    /// Determine of the [`Type`] is a [subtype](https://en.wikipedia.org/wiki/Subtyping) of `other`.
    ///
    /// This should only be used at parse-time.
    /// If you have a concrete [`Value`](crate::Value) or [`PipelineData`](crate::PipelineData),
    /// you should use their respective `is_subtype_of` methods instead.
    // This is identical to this method's default implementation. Written here to attach doccomment.
    fn is_subtype_of(&self, other: &Self) -> bool {
        matches!(
            self.compare_types(other),
            Some(TypeRelation::Subtype | TypeRelation::Equal)
        )
    }

    fn is_any(&self) -> bool {
        matches!(self, Type::Any)
    }

    fn is_assignable_to(&self, dst: &Self) -> bool {
        let src = self;
        match (dst, src) {
            (Type::Table(dst_cols), Type::List(src_ty))
                if let Type::Record(src_cols) = src_ty.as_ref() =>
            {
                src_cols.is_assignable_to(dst_cols)
            }
            (Type::List(dst_ty), Type::Table(src_cols))
                if let Type::Record(dst_cols) = dst_ty.as_ref() =>
            {
                src_cols.is_assignable_to(dst_cols)
            }
            (Type::Record(dst_cols), Type::Record(src_cols))
            | (Type::Table(dst_cols), Type::Table(src_cols)) => src_cols.is_assignable_to(dst_cols),
            // strings can be coerced globs
            (Type::Glob, Type::String) => true,
            // but not the other way around
            (Type::String, Type::Glob) => false,
            (Type::OneOf(dst_tys), Type::OneOf(src_tys)) => src_tys.is_assignable_to(dst_tys),
            (Type::OneOf(dst_tys), src_ty) => src_ty.is_assignable_to(dst_tys),
            (dst_ty, Type::OneOf(src_tys)) => src_tys.is_assignable_to(dst_ty),
            // leave it to the runtime
            (Type::List(_) | Type::Table(_) | Type::Record(_), Type::Custom(_)) => true,
            (lhs, rhs @ Type::CellPath) => rhs.is_subtype_of(lhs),
            (lhs, rhs) => rhs.compare_types(lhs).is_some(),
        }
    }
}

impl TypeSet for Type {
    fn union(self, other: Self) -> Self {
        let (lhs, rhs) = match Self::flat_widen(self, other) {
            Ok(t) => return t,
            Err(tys) => tys,
        };

        match (lhs, rhs) {
            (Type::OneOf(ts), Type::OneOf(us)) => Type::OneOf(ts.union(us)),
            (Type::OneOf(oneof), t) | (t, Type::OneOf(oneof)) => Type::OneOf(oneof.add_ty(t)),
            (this, other) => Type::one_of([this, other]),
        }
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Block => write!(f, "block"),
            Type::Closure => write!(f, "closure"),
            Type::Bool => write!(f, "bool"),
            Type::CellPath => write!(f, "cell-path"),
            Type::Date => write!(f, "datetime"),
            Type::Duration => write!(f, "duration"),
            Type::Filesize => write!(f, "filesize"),
            Type::Float => write!(f, "float"),
            Type::Int => write!(f, "int"),
            Type::Range => write!(f, "range"),
            Type::Record(columns) => write!(f, "record{columns}"),
            Type::Table(columns) => write!(f, "table{columns}"),
            Type::List(l) => write!(f, "list<{l}>"),
            Type::Nothing => write!(f, "nothing"),
            Type::Number => write!(f, "number"),
            Type::OneOf(oneof) => write!(f, "{oneof}"),
            Type::String => write!(f, "string"),
            Type::Any => write!(f, "any"),
            Type::Error => write!(f, "error"),
            Type::Binary => write!(f, "binary"),
            Type::Custom(custom) => write!(f, "{custom}"),
            Type::Glob => write!(f, "glob"),
        }
    }
}

/// Get a string nicely combining multiple types
///
/// Helpful for listing types in errors
pub fn combined_type_string<'a, I>(types: I, join_word: &str) -> Option<String>
where
    I: IntoIterator<Item = &'a Type>,
{
    use std::fmt::Write as _;

    // Deduplicate types to avoid confusing repeated entries like
    // "binary, binary, binary, or binary" in error messages.
    let mut seen = Vec::new();
    for t in types {
        if !seen.contains(t) {
            seen.push(t.clone());
        }
    }

    match seen.as_slice() {
        [] => None,
        [one] => Some(one.to_string()),
        [one, two] => Some(format!("{one} {join_word} {two}")),
        [initial @ .., last] => {
            let mut out = String::new();
            for ele in initial {
                let _ = write!(out, "{ele}, ");
            }
            let _ = write!(out, "{join_word} {last}");
            Some(out)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    mod subtype_relation {
        use super::*;

        #[test]
        fn test_reflexivity() {
            for ty in Type::iter() {
                assert!(ty.is_subtype_of(&ty));
            }
        }

        #[test]
        fn test_any_is_top_type() {
            for ty in Type::iter() {
                assert!(ty.is_subtype_of(&Type::Any));
            }
        }

        #[test]
        fn test_number_supertype() {
            assert!(Type::Int.is_subtype_of(&Type::Number));
            assert!(Type::Float.is_subtype_of(&Type::Number));
        }

        #[test]
        fn test_list_covariance() {
            for ty1 in Type::iter() {
                for ty2 in Type::iter() {
                    let list_ty1 = Type::List(Box::new(ty1.clone()));
                    let list_ty2 = Type::List(Box::new(ty2.clone()));
                    assert_eq!(list_ty1.is_subtype_of(&list_ty2), ty1.is_subtype_of(&ty2));
                }
            }
        }
    }

    mod oneof {
        use super::*;

        #[test]
        fn oneof_lhs() {
            let rel = Type::one_of([Type::Int, Type::Nothing]).compare_types(&Type::Int);
            assert_eq!(rel, Some(TypeRelation::Supertype));
        }

        #[test]
        fn oneof_rhs() {
            let rel = Type::Int.compare_types(&Type::one_of([Type::Int, Type::Nothing]));
            assert_eq!(rel, Some(TypeRelation::Subtype));
        }
    }

    mod oneof_flattening {
        use super::*;

        #[test]
        fn test_oneof_creation_flattens() {
            let nested = Type::one_of([
                Type::String,
                Type::one_of([Type::Int, Type::Float]),
                Type::Bool,
            ]);
            if let Type::OneOf(oneof) = nested {
                let types_vec: Vec<Type> = oneof.into_iter().collect();
                assert_eq!(types_vec.len(), 3);
                assert!(types_vec.contains(&Type::String));
                assert!(types_vec.contains(&Type::Number));
                assert!(types_vec.contains(&Type::Bool));
            } else {
                panic!("Expected OneOf");
            }
        }

        #[test]
        fn test_widen_flattens_oneof() {
            let a = Type::one_of([Type::String, Type::Int]);
            let b = Type::one_of([Type::Float, Type::Bool]);
            let widened = a.union(b);
            if let Type::OneOf(oneof) = widened {
                let types_vec: Vec<Type> = oneof.into_iter().collect();
                assert_eq!(types_vec.len(), 3);
                assert!(types_vec.contains(&Type::String));
                assert!(types_vec.contains(&Type::Number)); // Int + Float -> Number
                assert!(types_vec.contains(&Type::Bool));
            } else {
                panic!("Expected OneOf");
            }
        }

        #[test]
        fn test_oneof_deduplicates() {
            let record_type =
                Type::Record(vec![("content".to_string(), Type::list(Type::String))].into());
            let oneof = Type::one_of([Type::String, record_type.clone(), record_type.clone()]);
            if let Type::OneOf(oneof) = oneof {
                let types_vec: Vec<Type> = oneof.into_iter().collect();
                assert_eq!(types_vec.len(), 2);
                assert!(types_vec.contains(&Type::String));
                assert!(types_vec.contains(&record_type));
            } else {
                panic!("Expected OneOf");
            }
        }
    }

    // regressions and performance tests for the subtype shortcut added above
    mod widen_shortcuts {
        use super::*;

        #[test]
        fn test_widen_subtype_shortcut() {
            // widening a union that already covers the new type should return the original union unchanged.
            let union = Type::one_of([Type::String, Type::Number]);
            let result = union.clone().union(Type::Int);
            assert_eq!(result, union);

            // symmetric case where the left side is the subtype
            let union2 = Type::one_of([Type::Int, Type::String]);
            let result2 = Type::Int.union(union2.clone());
            assert_eq!(result2, union2);
        }

        #[test]
        fn test_chain_shortcut() {
            // repeatedly widen the same type pair
            let mut t = Type::String;
            for _ in 0..100 {
                t = t.union(Type::Int);
            }
            let expected = Type::one_of([Type::String, Type::Int]);
            assert_eq!(t, expected);
        }

        #[test]
        fn test_list_table_widen_preserves_list() {
            let list_record = Type::list(Type::Record(vec![("a".to_string(), Type::Int)].into()));
            let table = Type::Table(vec![("a".to_string(), Type::Int)].into());

            let widened = list_record.clone().union(table.clone());
            let expected = Type::one_of([list_record, table]);

            assert_eq!(widened, expected);
        }

        #[test]
        fn test_glob_string_union() {
            let g = Type::Glob;
            let s = Type::String;
            let w1 = g.clone().union(s.clone());
            let w2 = s.clone().union(g.clone());
            let expected1 = Type::one_of([Type::Glob, Type::String]);
            let expected2 = Type::one_of([Type::String, Type::Glob]);
            assert_eq!(w1, expected1);
            assert_eq!(w2, expected2);
        }
    }
}
