#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeRelation {
    /// Strict subtype
    Subtype,
    Equal,
    /// Strict supertype
    Supertype,
}

impl TypeRelation {
    pub fn reverse(self) -> Self {
        match self {
            Self::Subtype => Self::Supertype,
            Self::Equal => Self::Equal,
            Self::Supertype => Self::Subtype,
        }
    }

    /// Combines two partial relation results.
    ///
    /// While computing the relation of two types, sometimes it has to be done in multiple steps,
    /// e.g. `oneof`, `record`.
    /// -   In both cases, one may start with [`Self::Equal`] before processing any type parameters.
    /// -   If both types are empty (e.g. `record<>` with no columns, `oneof<>` with no variants),
    ///     [`Self::Equal`] is the correct result without requiring further checks.
    /// -   As items are checked, the result of these checks may not form a coherent result.
    ///     e.g.: `lhs = record<a: int, b: int>` and `rhs = record<a: int, c: int>`
    ///     -   both `lhs` and `rhs` have an `a: int` column. `lhs` and `rhs` might be [Equal](Self::Equal)
    ///     -   `lhs` has a `b: int` column, which `rhs` does not. This makes `lhs` the more
    ///         specific type between the two, thus `lhs` might be a [Subtype](Self::Subtype) of `rhs`
    ///     -   `lhs` does not have a `b: int` column, which `rhs` does. This makes `rhs` the more
    ///         specific type between the two, thus `lhs` might be a [Supertype](Self::Supertype) of `rhs`
    /// -   If there are conflicting results, returns [`None`]
    pub fn combine(self, other: Self) -> Option<Self> {
        match (self, other) {
            (s, Self::Equal) | (Self::Equal, s) => Some(s),
            (Self::Subtype, Self::Subtype) => Some(Self::Subtype),
            (Self::Supertype, Self::Supertype) => Some(Self::Supertype),
            _ => None,
        }
    }
}

/// Trait for comparisons corresponding to subtyping relations.
///
/// `<:` and `:>` are used to mean "is subtype of" and "is supertype of" respectively.
/// `<<:` and `:>>` are used as their "strict" counterparts (no equality).
///
/// Implementations must be:
/// - *Transitive*: if `a <: b` and `b <: c`, then `a <: c`
/// - *Dual*: if `a <: b`, then `b :> a`
/// - *Antisymmetric*:
///   - if `a <<: b`, then `b <<: a` can't be true
///   - if `a <: b` and `b <: a`, then `a == b`
///
/// Output of [`Self::is_subtype_of`] and [`Self::is_supertype_of`] must be consistent with
/// [`Self::compare_types`]
///
/// The `Rhs` type parameter is to allow comparing the runtime type of a value with static types,
/// without going through intermediate steps.
/// Instead of `Value::get_type(&val).compare_types(&ty)`, which might require allocations for the
/// intermediate `Type`, one can use `Value::compare_types(&val, &ty)`
pub trait CompareTypes<Rhs = Self> {
    /// Compares types and returns their relation with regards to subtyping.
    ///
    /// - Returns `Some` if types are equal, or one type is a subtype of the other.
    /// - Returns `None` if neither type is equal to or is subtype of the other.
    fn compare_types(&self, other: &Rhs) -> Option<TypeRelation>;

    /// Returns `true` if `self` is equal to or is a *strict* subtype of `other`
    ///
    /// Converse of [`Self::is_supertype_of`]
    fn is_subtype_of(&self, other: &Rhs) -> bool {
        matches!(
            self.compare_types(other),
            Some(TypeRelation::Subtype | TypeRelation::Equal)
        )
    }

    /// Returns `true` if `self` is equal to or is a *strict* supertype of `other`
    ///
    /// Converse of [`Self::is_subtype_of`]
    fn is_supertype_of(&self, other: &Rhs) -> bool {
        matches!(
            self.compare_types(other),
            Some(TypeRelation::Supertype | TypeRelation::Equal)
        )
    }

    /// Allows having an "escape hatch".
    ///
    /// Due to not having separate top and bottom types, and treating `any` as both, we need to be
    /// especially lax in some situations to keep things convenient and backwards compatible.
    ///
    /// We can't treat `any` as a bottom type in [`CompareTypes::compare_types`] as due to
    /// reflexivity it would imply all types are subtypes of all other types:
    /// - `any` as top type: `int <: any`
    /// - `any` as *bottom* type: `any <: list`
    /// - `int <: list`???
    fn is_any(&self) -> bool {
        false
    }

    /// Equivalent to [`CompareTypes::is_subtype_of`] by default.
    ///
    /// Exists as a separate method to allow relaxing requirements when needed.
    fn is_assignable_to(&self, dst: &Rhs) -> bool {
        self.is_subtype_of(dst)
    }
}

/// Trait for set operations on types.
pub trait TypeSet {
    /// Returns the narrowest common supertype of the given types.
    #[doc(alias = "widen")]
    fn union(self, other: Self) -> Self;

    // // no actual use case yet. this is just where it should be when/if it's implemented
    // #[doc(alias = "narrow")]
    // fn intersection(self, other: Self) -> Option<Self>;
}
