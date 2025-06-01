use syn::{Attribute, Fields, LitStr, meta::ParseNestedMeta, spanned::Spanned};

use crate::{HELPER_ATTRIBUTE, case::Case, error::DeriveError};

pub trait ParseAttrs: Default {
    fn parse_attrs<'a, M>(
        iter: impl IntoIterator<Item = &'a Attribute>,
    ) -> Result<Self, DeriveError<M>> {
        let mut attrs = Self::default();
        for attr in filter(iter.into_iter()) {
            // This is a container to allow returning derive errors inside the parse_nested_meta fn.
            let mut err = Ok(());
            let _ = attr.parse_nested_meta(|meta| {
                attrs.parse_attr(meta).or_else(|e| {
                    err = Err(e);
                    Ok(()) // parse_nested_meta requires another error type, so we escape it here
                })
            });
            err?; // Shortcircuit here if `err` is holding some error.
        }

        Ok(attrs)
    }

    fn parse_attr<M>(&mut self, attr_meta: ParseNestedMeta<'_>) -> Result<(), DeriveError<M>>;
}

#[derive(Debug, Default)]
pub struct ContainerAttributes {
    pub rename_all: Option<Case>,
    pub type_name: Option<String>,
}

impl ParseAttrs for ContainerAttributes {
    fn parse_attr<M>(&mut self, attr_meta: ParseNestedMeta<'_>) -> Result<(), DeriveError<M>> {
        let ident = attr_meta.path.require_ident()?;
        match ident.to_string().as_str() {
            "rename_all" => {
                let case: LitStr = attr_meta.value()?.parse()?;
                let value_span = case.span();
                let case = case.value();
                match Case::from_str(&case) {
                    Some(case) => self.rename_all = Some(case),
                    None => {
                        return Err(DeriveError::InvalidAttributeValue {
                            value_span,
                            value: Box::new(case),
                        });
                    }
                }
            }
            "type_name" => {
                let type_name: LitStr = attr_meta.value()?.parse()?;
                let type_name = type_name.value();
                self.type_name = Some(type_name);
            }
            ident => {
                return Err(DeriveError::UnexpectedAttribute {
                    meta_span: ident.span(),
                });
            }
        }

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct MemberAttributes {
    pub rename: Option<String>,
    pub default: bool,
}

impl ParseAttrs for MemberAttributes {
    fn parse_attr<M>(&mut self, attr_meta: ParseNestedMeta<'_>) -> Result<(), DeriveError<M>> {
        let ident = attr_meta.path.require_ident()?;
        match ident.to_string().as_str() {
            "rename" => {
                let rename: LitStr = attr_meta.value()?.parse()?;
                let rename = rename.value();
                self.rename = Some(rename);
            }
            "default" => {
                self.default = true;
            }
            ident => {
                return Err(DeriveError::UnexpectedAttribute {
                    meta_span: ident.span(),
                });
            }
        }

        Ok(())
    }
}

pub fn filter<'a>(
    iter: impl Iterator<Item = &'a Attribute>,
) -> impl Iterator<Item = &'a Attribute> {
    iter.filter(|attr| attr.path().is_ident(HELPER_ATTRIBUTE))
}

// The deny functions are built to easily deny the use of the helper attribute if used incorrectly.
// As the usage of it gets more complex, these functions might be discarded or replaced.

/// Deny any attribute that uses the helper attribute.
pub fn deny<M>(attrs: &[Attribute]) -> Result<(), DeriveError<M>> {
    match filter(attrs.iter()).next() {
        Some(attr) => Err(DeriveError::InvalidAttributePosition {
            attribute_span: attr.span(),
        }),
        None => Ok(()),
    }
}

/// Deny any attributes that uses the helper attribute on any field.
pub fn deny_fields<M>(fields: &Fields) -> Result<(), DeriveError<M>> {
    match fields {
        Fields::Named(fields) => {
            for field in fields.named.iter() {
                deny(&field.attrs)?;
            }
        }
        Fields::Unnamed(fields) => {
            for field in fields.unnamed.iter() {
                deny(&field.attrs)?;
            }
        }
        Fields::Unit => (),
    }

    Ok(())
}
