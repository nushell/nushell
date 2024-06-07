use convert_case::Case;
use syn::{spanned::Spanned, Attribute, Fields, LitStr};

use crate::{error::DeriveError, HELPER_ATTRIBUTE};

#[derive(Debug)]
pub struct ContainerAttributes {
    pub rename_all: Case,
}

impl Default for ContainerAttributes {
    fn default() -> Self {
        Self {
            rename_all: Case::Snake,
        }
    }
}

impl ContainerAttributes {
    pub fn parse_attrs<'a, M>(
        iter: impl Iterator<Item = &'a Attribute>,
    ) -> Result<Self, DeriveError<M>> {
        let mut container_attrs = ContainerAttributes::default();
        for attr in filter(iter) {
            // This is a container to allow returning derive errors inside the parse_nested_meta fn.
            let mut err = Ok(());

            attr.parse_nested_meta(|meta| {
                let ident = meta.path.require_ident()?;
                match ident.to_string().as_str() {
                    "rename_all" => {
                        let case: LitStr = meta.value()?.parse()?;
                        let case = match case.value().as_str() {
                            "UPPER CASE" => Case::Upper,
                            "lower case" => Case::Lower,
                            "Title Case" => Case::Title,
                            "tOGGLE cASE" => Case::Toggle,
                            "camelCase" => Case::Camel,
                            "PascalCase" | "UpperCamelCase" => Case::Pascal,
                            "snake_case" => Case::Snake,
                            "UPPER_SNAKE_CASE" | "SCREAMING_SNAKE_CASE" => Case::UpperSnake,
                            "kebab-case" => Case::Kebab,
                            "COBOL-CASE" | "UPPER-KEBAB-CASE" => Case::Cobol,
                            "Train-Case" => Case::Train,
                            "flatcase" => Case::Flat,
                            "UPPERFLATCASE" => Case::UpperFlat,
                            "aLtErNaTiNg CaSe" => Case::Alternating,
                            c => {
                                err = Err(DeriveError::InvalidAttributeValue {
                                    value_span: case.span(),
                                    value: Box::new(c.to_string()),
                                });
                                return Ok(()); // We stored the err in `err`.
                            }
                        };
                        container_attrs.rename_all = case;
                    }
                    ident => {
                        err = Err(DeriveError::UnexpectedAttribute {
                            meta_span: ident.span(),
                        });
                    }
                }

                Ok(())
            })
            .map_err(DeriveError::Syn)?;

            err?; // Shortcircuit here if `err` is holding some error.
        }

        Ok(container_attrs)
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
