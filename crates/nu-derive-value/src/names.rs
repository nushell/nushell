use proc_macro2::Span;
use std::collections::HashMap;
use syn::ext::IdentExt;
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

    /// Resolves an identifier using attributes and ensures its uniqueness.
    ///
    /// The identifier is transformed according to these rules:
    /// - If [`MemberAttributes::rename`] is set, this explicitly renamed value is used.
    ///   The value is defined by the helper attribute `#[nu_value(rename = "...")]` on a member.
    /// - If the above is not set but [`ContainerAttributes::rename_all`] is, the identifier
    ///   undergoes case conversion as specified by the helper attribute
    ///   `#[nu_value(rename_all = "...")]` on the container (struct or enum).
    /// - If neither renaming attribute is set, the function applies the case conversion provided
    ///   by the `default` parameter.
    ///   If `default` is `None`, the identifier remains unchanged.
    ///
    /// This function checks the transformed identifier against previously seen identifiers to
    /// ensure it is unique.
    /// If a duplicate identifier is detected, it returns [`DeriveError::NonUniqueName`].
    pub fn resolve_ident<M>(
        &mut self,
        ident: &Ident,
        container_attrs: &ContainerAttributes,
        member_attrs: &MemberAttributes,
        default: impl Into<Option<Case>>,
    ) -> Result<String, DeriveError<M>> {
        let span = ident.span();
        let ident = if let Some(rename) = &member_attrs.rename {
            rename.clone()
        } else {
            let case = container_attrs.rename_all.or(default.into());
            ident.unraw().to_case(case)
        };

        if let Some(seen) = self.seen_names.get(&ident) {
            return Err(DeriveError::NonUniqueName {
                name: ident.to_string(),
                first: *seen,
                second: span,
            });
        }

        self.seen_names.insert(ident.clone(), span);
        Ok(ident)
    }
}
