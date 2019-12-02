use nu_source::{DebugDocBuilder, HasSpan, Spanned, SpannedItem, Tagged};

pub trait ShellTypeName {
    fn type_name(&self) -> &'static str;
}

impl<T: ShellTypeName> ShellTypeName for Spanned<T> {
    fn type_name(&self) -> &'static str {
        self.item.type_name()
    }
}

impl<T: ShellTypeName> ShellTypeName for &T {
    fn type_name(&self) -> &'static str {
        (*self).type_name()
    }
}

pub trait SpannedTypeName {
    fn spanned_type_name(&self) -> Spanned<&'static str>;
}

impl<T: ShellTypeName + HasSpan> SpannedTypeName for T {
    fn spanned_type_name(&self) -> Spanned<&'static str> {
        self.type_name().spanned(self.span())
    }
}

impl<T: ShellTypeName> SpannedTypeName for Tagged<T> {
    fn spanned_type_name(&self) -> Spanned<&'static str> {
        self.item.type_name().spanned(self.tag.span)
    }
}

pub trait PrettyType {
    fn pretty_type(&self) -> DebugDocBuilder;
}
