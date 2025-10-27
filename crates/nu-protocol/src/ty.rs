use crate::SyntaxShape;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
#[cfg(test)]
use strum_macros::EnumIter;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Hash)]
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
    OneOf(Box<[Type]>),
    Range,
    Record(Box<[(String, Type)]>),
    String,
    Glob,
    Table(Box<[(String, Type)]>),
}

impl Type {
    pub fn list(inner: Type) -> Self {
        Self::List(Box::new(inner))
    }

    pub fn one_of(types: impl IntoIterator<Item = Type>) -> Self {
        Self::OneOf(types.into_iter().collect())
    }

    pub fn record() -> Self {
        Self::Record([].into())
    }

    pub fn table() -> Self {
        Self::Table([].into())
    }

    pub fn custom(name: impl Into<Box<str>>) -> Self {
        Self::Custom(name.into())
    }

    /// Determine of the [`Type`] is a [subtype](https://en.wikipedia.org/wiki/Subtyping) of `other`.
    ///
    /// This should only be used at parse-time.
    /// If you have a concrete [`Value`](crate::Value) or [`PipelineData`](crate::PipelineData),
    /// you should use their respective `is_subtype_of` methods instead.
    pub fn is_subtype_of(&self, other: &Type) -> bool {
        // Structural subtyping
        let is_subtype_collection = |this: &[(String, Type)], that: &[(String, Type)]| {
            if this.is_empty() || that.is_empty() {
                true
            } else if this.len() < that.len() {
                false
            } else {
                that.iter().all(|(col_y, ty_y)| {
                    if let Some((_, ty_x)) = this.iter().find(|(col_x, _)| col_x == col_y) {
                        ty_x.is_subtype_of(ty_y)
                    } else {
                        false
                    }
                })
            }
        };

        match (self, other) {
            (t, u) if t == u => true,
            (_, Type::Any) => true,
            // We want `get`/`select`/etc to accept string and int values, so it's convenient to
            // use them with variables, without having to explicitly convert them into cell-paths
            (Type::String | Type::Int, Type::CellPath) => true,
            (Type::OneOf(oneof), Type::CellPath) => {
                oneof.iter().all(|t| t.is_subtype_of(&Type::CellPath))
            }
            (Type::Float | Type::Int, Type::Number) => true,
            (Type::Glob, Type::String) | (Type::String, Type::Glob) => true,
            (Type::List(t), Type::List(u)) if t.is_subtype_of(u) => true, // List is covariant
            (Type::Record(this), Type::Record(that)) | (Type::Table(this), Type::Table(that)) => {
                is_subtype_collection(this, that)
            }
            (Type::Table(_), Type::List(that)) if matches!(**that, Type::Any) => true,
            (Type::Table(this), Type::List(that)) => {
                matches!(that.as_ref(), Type::Record(that) if is_subtype_collection(this, that))
            }
            (Type::List(this), Type::Table(that)) => {
                matches!(this.as_ref(), Type::Record(this) if is_subtype_collection(this, that))
            }
            (Type::OneOf(this), that @ Type::OneOf(_)) => {
                this.iter().all(|t| t.is_subtype_of(that))
            }
            (this, Type::OneOf(that)) => that.iter().any(|t| this.is_subtype_of(t)),
            _ => false,
        }
    }

    /// Returns the supertype between `self` and `other`, or `Type::Any` if they're unrelated
    pub fn widen(self, other: Type) -> Type {
        /// Returns supertype of arguments without creating a `oneof`, or falling back to `any`
        /// (unless one or both of the arguments are `any`)
        fn flat_widen(lhs: Type, rhs: Type) -> Result<Type, (Type, Type)> {
            Ok(match (lhs, rhs) {
                (lhs, rhs) if lhs == rhs => lhs,
                (Type::Any, _) | (_, Type::Any) => Type::Any,
                // (int, int) and (float, float) cases are already handled by the first match arm
                (
                    Type::Int | Type::Float | Type::Number,
                    Type::Int | Type::Float | Type::Number,
                ) => Type::Number,

                (Type::Glob, Type::String) | (Type::String, Type::Glob) => Type::String,
                (Type::Record(this), Type::Record(that)) => {
                    Type::Record(widen_collection(this, that))
                }
                (Type::Table(this), Type::Table(that)) => Type::Table(widen_collection(this, that)),
                (Type::List(list_item), Type::Table(table))
                | (Type::Table(table), Type::List(list_item)) => {
                    let item = match *list_item {
                        Type::Record(record) => Type::Record(widen_collection(record, table)),
                        list_item => Type::one_of([list_item, Type::Record(table)]),
                    };
                    Type::List(Box::new(item))
                }
                (Type::List(lhs), Type::List(rhs)) => Type::list(lhs.widen(*rhs)),
                (t, u) => return Err((t, u)),
            })
        }
        fn widen_collection(
            lhs: Box<[(String, Type)]>,
            rhs: Box<[(String, Type)]>,
        ) -> Box<[(String, Type)]> {
            if lhs.is_empty() || rhs.is_empty() {
                return [].into();
            }
            let (small, big) = match lhs.len() <= rhs.len() {
                true => (lhs, rhs),
                false => (rhs, lhs),
            };
            small
                .into_iter()
                .filter_map(|(col, typ)| {
                    big.iter()
                        .find_map(|(b_col, b_typ)| (&col == b_col).then(|| b_typ.clone()))
                        .map(|b_typ| (col, typ, b_typ))
                })
                .map(|(col, t, u)| (col, t.widen(u)))
                .collect()
        }

        fn oneof_add(oneof: &mut Vec<Type>, mut t: Type) {
            if oneof.contains(&t) {
                return;
            }

            for one in oneof.iter_mut() {
                match flat_widen(std::mem::replace(one, Type::Any), t) {
                    Ok(one_t) => {
                        *one = one_t;
                        return;
                    }
                    Err((one_, t_)) => {
                        *one = one_;
                        t = t_;
                    }
                }
            }

            oneof.push(t);
        }

        let tu = match flat_widen(self, other) {
            Ok(t) => return t,
            Err(tu) => tu,
        };

        match tu {
            (Type::OneOf(ts), Type::OneOf(us)) => {
                let (big, small) = match ts.len() >= us.len() {
                    true => (ts, us),
                    false => (us, ts),
                };
                let mut out = big.into_vec();
                for t in small.into_iter() {
                    oneof_add(&mut out, t);
                }
                Type::one_of(out)
            }
            (Type::OneOf(oneof), t) | (t, Type::OneOf(oneof)) => {
                let mut out = oneof.into_vec();
                oneof_add(&mut out, t);
                Type::one_of(out)
            }
            (this, other) => Type::one_of([this, other]),
        }
    }

    /// Returns the supertype of all types within `it`. Short-circuits on, and falls back to, `Type::Any`.
    pub fn supertype_of(it: impl IntoIterator<Item = Type>) -> Option<Self> {
        let mut it = it.into_iter();
        it.next().and_then(|head| {
            it.try_fold(head, |acc, e| match acc.widen(e) {
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
        let mk_shape = |tys: &[(String, Type)]| {
            tys.iter()
                .map(|(key, val)| (key.clone(), val.to_shape()))
                .collect()
        };

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
            Type::Record(entries) => SyntaxShape::Record(mk_shape(entries)),
            Type::Table(columns) => SyntaxShape::Table(mk_shape(columns)),
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
            Type::Record(fields) => {
                if fields.is_empty() {
                    write!(f, "record")
                } else {
                    write!(
                        f,
                        "record<{}>",
                        fields
                            .iter()
                            .map(|(x, y)| format!("{x}: {y}"))
                            .collect::<Vec<String>>()
                            .join(", "),
                    )
                }
            }
            Type::Table(columns) => {
                if columns.is_empty() {
                    write!(f, "table")
                } else {
                    write!(
                        f,
                        "table<{}>",
                        columns
                            .iter()
                            .map(|(x, y)| format!("{x}: {y}"))
                            .collect::<Vec<String>>()
                            .join(", ")
                    )
                }
            }
            Type::List(l) => write!(f, "list<{l}>"),
            Type::Nothing => write!(f, "nothing"),
            Type::Number => write!(f, "number"),
            Type::OneOf(types) => {
                write!(f, "oneof")?;
                let [first, rest @ ..] = &**types else {
                    return Ok(());
                };
                write!(f, "<{first}")?;
                for t in rest {
                    write!(f, ", {t}")?;
                }
                f.write_str(">")
            }
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
pub fn combined_type_string(types: &[Type], join_word: &str) -> Option<String> {
    use std::fmt::Write as _;
    match types {
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
    use super::Type;
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
}
