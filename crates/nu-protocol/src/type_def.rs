use crate::SyntaxShape;

/// A named type declared with the `type` keyword.
///
/// Named types are stored in [`OverlayFrame`](crate::OverlayFrame)s and resolved
/// by name at parse time. Because they live in overlay frames, they obey the
/// same scoping and shadowing rules as modules.
#[derive(Debug, Clone)]
pub struct TypeDef {
    /// The name the type was declared under.
    pub name: Vec<u8>,
    pub kind: TypeDefKind,
}

#[derive(Debug, Clone)]
pub enum TypeDefKind {
    /// `type Name = <shape>` — a structural alias for another type.
    /// The alias is interchangeable with the aliased type everywhere.
    Alias(SyntaxShape),
    /// `type Name = enum<...>` — a nominal sum type. Values carry the declared
    /// name as their type identity (via [`Type::Custom`](crate::Type::Custom)).
    Enum(EnumDef),
}

/// The variants of a declared `enum` type.
#[derive(Debug, Clone)]
pub struct EnumDef {
    pub variants: Vec<EnumVariant>,
}

impl EnumDef {
    pub fn get_variant(&self, name: &str) -> Option<&EnumVariant> {
        self.variants.iter().find(|v| v.name == name)
    }

    pub fn variant_names(&self) -> Vec<String> {
        self.variants.iter().map(|v| v.name.clone()).collect()
    }
}

/// A single variant of an `enum` type declaration.
#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    /// The payload shape for the variant, or `None` for unit variants
    /// (e.g. `point` in `enum<point, circle: record<radius: float>>`).
    pub payload: Option<SyntaxShape>,
}
