use super::ExprDb;
use nu_protocol::{ast::PathMember, CustomValue, PipelineData, ShellError, Span, Value};
use serde::{Deserialize, Serialize};
use sqlparser::ast::{Expr, Ident, SelectItem};

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
    fn from(expr: SelectItem) -> Self {
        Self(expr)
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

        SelectDb::select_to_value(self.as_ref(), span).follow_cell_path(&[path])
    }

    fn follow_path_string(&self, column_name: String, span: Span) -> Result<Value, ShellError> {
        let path = PathMember::String {
            val: column_name,
            span,
        };
        SelectDb::select_to_value(self.as_ref(), span).follow_cell_path(&[path])
    }

    fn typetag_name(&self) -> &'static str {
        "DB selection"
    }

    fn typetag_deserialize(&self) {
        unimplemented!("typetag_deserialize")
    }
}

impl SelectDb {
    pub fn try_from_value(value: Value) -> Result<Self, ShellError> {
        match value {
            Value::CustomValue { val, span } => match val.as_any().downcast_ref::<Self>() {
                Some(expr) => Ok(Self(expr.0.clone())),
                None => Err(ShellError::CantConvert(
                    "db expression".into(),
                    "non-expression".into(),
                    span,
                    None,
                )),
            },
            x => Err(ShellError::CantConvert(
                "database".into(),
                x.get_type().to_string(),
                x.span()?,
                None,
            )),
        }
    }

    pub fn try_from_pipeline(input: PipelineData, span: Span) -> Result<Self, ShellError> {
        let value = input.into_value(span);
        Self::try_from_value(value)
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
                    val: format!("{}", alias.value),
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
    pub fn into_selects(value: Value) -> Result<Vec<SelectItem>, ShellError> {
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
            Value::CustomValue { .. } => SelectDb::try_from_value(value)
                .map(SelectDb::into_native)
                .map(ExtractedSelect::Single),
            Value::List { vals, .. } => vals
                .into_iter()
                .map(Self::extract_selects)
                .collect::<Result<Vec<ExtractedSelect>, ShellError>>()
                .map(ExtractedSelect::List),
            x => Err(ShellError::CantConvert(
                "expression".into(),
                x.get_type().to_string(),
                x.span()?,
                None,
            )),
        }
    }
}
