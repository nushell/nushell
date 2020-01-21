use nu_source::{DebugDocBuilder, HasSpan, Spanned, SpannedItem, Tagged};

/// A trait that allows structures to define a known .type_name() which pretty-prints the type
pub trait ShellTypeName {
    fn type_name(&self) -> &'static str;
}

impl<T: ShellTypeName> ShellTypeName for Spanned<T> {
    /// Return the type_name of the spanned item
    fn type_name(&self) -> &'static str {
        self.item.type_name()
    }
}

impl<T: ShellTypeName> ShellTypeName for &T {
    /// Return the type_name for the borrowed reference
    fn type_name(&self) -> &'static str {
        (*self).type_name()
    }
}

/// A trait that allows structures to define a known way to return a spanned type name
pub trait SpannedTypeName {
    fn spanned_type_name(&self) -> Spanned<&'static str>;
}

impl<T: ShellTypeName + HasSpan> SpannedTypeName for T {
    /// Return the type name as a spanned string
    fn spanned_type_name(&self) -> Spanned<&'static str> {
        self.type_name().spanned(self.span())
    }
}

impl<T: ShellTypeName> SpannedTypeName for Tagged<T> {
    /// Return the spanned type name for a Tagged value
    fn spanned_type_name(&self) -> Spanned<&'static str> {
        self.item.type_name().spanned(self.tag.span)
    }
}

/// A trait to enable pretty-printing of type information
pub trait PrettyType {
    fn pretty_type(&self) -> DebugDocBuilder;
}
