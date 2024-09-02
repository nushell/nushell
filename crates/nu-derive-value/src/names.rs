use proc_macro2::Span;
use std::collections::HashMap;
use syn::Ident;

use crate::attributes::{ContainerAttributes, MemberAttributes};
use crate::case::{Case, Casing};
use crate::error::DeriveError;

#[derive(Debug, Default)]
pub struct NameResolver {
    seen_names: HashMap<String, Span>,
}

impl NameResolver {
    pub fn new() -> Self {
        Self::default()
    }

    /// Resolve identifier with attributes.
    /// 
    /// This method resolves the name that should be used in the `Value`.
    /// By remembering which idents came before, we can check that every ident is unique.
    /// If that is not the case, we return a [`DeriveError::NonUniqueName`].
    pub fn resolve_ident<M>(
        &mut self,
        ident: &'_ Ident,
        container_attrs: &'_ ContainerAttributes,
        member_attrs: &'_ MemberAttributes,
        default: impl Into<Option<Case>>,
    ) -> Result<String, DeriveError<M>> {
        let span = ident.span();
        let rename_all = container_attrs.rename_all;
        let rename = member_attrs.rename.as_ref();
        let ident = match (rename, rename_all) {
            (Some(rename), _) => rename.to_string(),
            (None, Some(case)) => ident.to_case(case),
            (None, None) => ident.to_case(default),
        };

        if let Some(seen) = self.seen_names.get(&ident) {
            return Err(DeriveError::NonUniqueName { 
                name: ident.to_string(), 
                first: *seen, 
                second: span 
            });
        }

        self.seen_names.insert(ident.clone(), span);
        Ok(ident)
    }
}
