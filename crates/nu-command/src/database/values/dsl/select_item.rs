use super::ExprDb;
use nu_protocol::{ast::PathMember, CustomValue, ShellError, Span, Value};
use serde::{Deserialize, Serialize};
use sqlparser::ast::{Expr, Ident, ObjectName, SelectItem};

#[derive(Debug, Serialize, Deserialize)]
pub struct SelectDb(SelectItem);

// Referenced access to the native expression
impl AsRef<SelectItem> for SelectDb {
    fn as_ref(&self) -> &SelectItem {
        &self.0
    }
}

impl AsMut<SelectItem> for SelectDb {
    fn as_mut(&mut self) -> &mut SelectItem {
        &mut self.0
    }
}

impl From<SelectItem> for SelectDb {
    fn from(selection: SelectItem) -> Self {
        Self(selection)
    }
}

impl From<Expr> for SelectDb {
    fn from(expr: Expr) -> Self {
        SelectItem::UnnamedExpr(expr).into()
    }
}

impl CustomValue for SelectDb {
    fn clone_value(&self, span: Span) -> Value {
        let cloned = Self(self.0.clone());

        Value::CustomValue {
            val: Box::new(cloned),
            span,
        }
    }

    fn value_string(&self) -> String {
        self.typetag_name().to_string()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        Ok(self.to_value(span))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn follow_path_int(&self, count: usize, span: Span) -> Result<Value, ShellError> {
        let path = PathMember::Int { val: count, span };

        SelectDb::select_to_value(self.as_ref(), span).follow_cell_path(&[path], false)
    }

    fn follow_path_string(&self, column_name: String, span: Span) -> Result<Value, ShellError> {
        let path = PathMember::String {
            val: column_name,
            span,
        };
        SelectDb::select_to_value(self.as_ref(), span).follow_cell_path(&[path], false)
    }

    fn typetag_name(&self) -> &'static str {
        "DB selection"
    }

    fn typetag_deserialize(&self) {
        unimplemented!("typetag_deserialize")
    }
}

impl SelectDb {
    pub fn try_from_value(value: &Value) -> Result<Self, ShellError> {
        match value {
            Value::CustomValue { val, span } => match val.as_any().downcast_ref::<Self>() {
                Some(expr) => Ok(Self(expr.0.clone())),
                None => Err(ShellError::CantConvert(
                    "db selection".into(),
                    "non-expression".into(),
                    *span,
                    None,
                )),
            },
            Value::String { val, .. } => match val.as_str() {
                "*" => Ok(SelectItem::Wildcard.into()),
                name if (name.contains('.') && name.contains('*')) => {
                    let parts: Vec<Ident> = name
                        .split('.')
                        .filter(|part| part != &"*")
                        .map(|part| Ident {
                            value: part.to_string(),
                            quote_style: None,
                        })
                        .collect();

                    Ok(SelectItem::QualifiedWildcard(ObjectName(parts)).into())
                }
                name if name.contains('.') => {
                    let parts: Vec<Ident> = name
                        .split('.')
                        .map(|part| Ident {
                            value: part.to_string(),
                            quote_style: None,
                        })
                        .collect();

                    let expr = Expr::CompoundIdentifier(parts);
                    Ok(SelectItem::UnnamedExpr(expr).into())
                }
                _ => {
                    let expr = Expr::Identifier(Ident {
                        value: val.clone(),
                        quote_style: None,
                    });

                    Ok(SelectItem::UnnamedExpr(expr).into())
                }
            },
            x => Err(ShellError::CantConvert(
                "selection".into(),
                x.get_type().to_string(),
                x.span()?,
                None,
            )),
        }
    }

    pub fn into_value(self, span: Span) -> Value {
        Value::CustomValue {
            val: Box::new(self),
            span,
        }
    }

    pub fn into_native(self) -> SelectItem {
        self.0
    }

    pub fn to_value(&self, span: Span) -> Value {
        SelectDb::select_to_value(self.as_ref(), span)
    }
}

impl SelectDb {
    fn select_to_value(select: &SelectItem, span: Span) -> Value {
        match select {
            SelectItem::UnnamedExpr(expr) => ExprDb::expr_to_value(expr, span),
            SelectItem::ExprWithAlias { expr, alias } => {
                let expr = ExprDb::expr_to_value(expr, span);

                let val = Value::String {
                    val: alias.value.to_string(),
                    span,
                };
                let style = Value::String {
                    val: format!("{:?}", alias.quote_style),
                    span,
                };

                let cols = vec!["value".into(), "quoted_style".into()];
                let alias = Value::Record {
                    cols,
                    vals: vec![val, style],
                    span,
                };

                let cols = vec!["expression".into(), "alias".into()];
                Value::Record {
                    cols,
                    vals: vec![expr, alias],
                    span,
                }
            }
            SelectItem::QualifiedWildcard(object) => {
                let vals: Vec<Value> = object
                    .0
                    .iter()
                    .map(|ident| Value::String {
                        val: ident.value.clone(),
                        span,
                    })
                    .collect();

                Value::List { vals, span }
            }
            SelectItem::Wildcard => Value::String {
                val: "*".into(),
                span,
            },
        }
    }

    // Convenient function to extrac multiple SelectItem that could be inside a
    // nushell Value
    pub fn extract_selects(value: Value) -> Result<Vec<SelectItem>, ShellError> {
        ExtractedSelect::extract_selects(value).map(ExtractedSelect::into_selects)
    }
}

// Enum to represent the parsing of the selects from Value
enum ExtractedSelect {
    Single(SelectItem),
    List(Vec<ExtractedSelect>),
}

impl ExtractedSelect {
    fn into_selects(self) -> Vec<SelectItem> {
        match self {
            Self::Single(select) => vec![select],
            Self::List(selects) => selects
                .into_iter()
                .flat_map(ExtractedSelect::into_selects)
                .collect(),
        }
    }

    fn extract_selects(value: Value) -> Result<ExtractedSelect, ShellError> {
        match value {
            Value::String { val, .. } => {
                let expr = Expr::Identifier(Ident {
                    value: val,
                    quote_style: None,
                });

                Ok(ExtractedSelect::Single(SelectItem::UnnamedExpr(expr)))
            }
            Value::CustomValue { .. } => {
                if let Ok(expr) = ExprDb::try_from_value(&value) {
                    Ok(ExtractedSelect::Single(SelectItem::UnnamedExpr(
                        expr.into_native(),
                    )))
                } else if let Ok(select) = SelectDb::try_from_value(&value) {
                    Ok(ExtractedSelect::Single(select.into_native()))
                } else {
                    Err(ShellError::CantConvert(
                        "selection".into(),
                        value.get_type().to_string(),
                        value.span()?,
                        None,
                    ))
                }
            }
            Value::List { vals, .. } => vals
                .into_iter()
                .map(Self::extract_selects)
                .collect::<Result<Vec<ExtractedSelect>, ShellError>>()
                .map(ExtractedSelect::List),
            x => Err(ShellError::CantConvert(
                "selection".into(),
                x.get_type().to_string(),
                x.span()?,
                None,
            )),
        }
    }
}
