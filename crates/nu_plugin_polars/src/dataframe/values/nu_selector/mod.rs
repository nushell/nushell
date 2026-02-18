mod custom_value;

use nu_protocol::{ShellError, Span, Value};
use polars::prelude::{Expr, PlSmallStr, Selector};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

pub use self::custom_value::NuSelectorCustomValue;

use super::{CustomValueSupport, NuExpression, PolarsPluginObject, PolarsPluginType};
use crate::{Cacheable, PolarsPlugin, values::NuExpressionCustomValue};

#[derive(Default, Clone, Debug)]
pub struct NuSelector {
    pub id: Uuid,
    selector: Option<Selector>,
}

// Mocked serialization (Selectors may not be serializable)
impl Serialize for NuSelector {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_none()
    }
}

impl<'de> Deserialize<'de> for NuSelector {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(NuSelector::default())
    }
}

impl AsRef<Selector> for NuSelector {
    fn as_ref(&self) -> &Selector {
        self.selector
            .as_ref()
            .expect("selector should always exist")
    }
}

impl From<Selector> for NuSelector {
    fn from(selector: Selector) -> Self {
        Self::new(Some(selector))
    }
}

impl NuSelector {
    fn new(selector: Option<Selector>) -> Self {
        Self {
            id: Uuid::new_v4(),
            selector,
        }
    }

    pub fn into_polars(self) -> Selector {
        self.selector.expect("Selector cannot be none to convert")
    }

    pub fn into_expr(self) -> NuExpression {
        NuExpression::from(Expr::Selector(self.into_polars()))
    }

    pub fn to_value(&self, span: Span) -> Result<Value, ShellError> {
        // Convert selector to a displayable string representation
        Ok(Value::string(format!("{:?}", self.selector), span))
    }
}

impl Cacheable for NuSelector {
    fn cache_id(&self) -> &Uuid {
        &self.id
    }

    fn to_cache_value(&self) -> Result<PolarsPluginObject, ShellError> {
        Ok(PolarsPluginObject::NuSelector(self.clone()))
    }

    fn from_cache_value(cv: PolarsPluginObject) -> Result<Self, ShellError> {
        match cv {
            PolarsPluginObject::NuSelector(selector) => Ok(selector),
            _ => Err(ShellError::GenericError {
                error: "Cache value is not a selector".into(),
                msg: "".into(),
                span: None,
                help: None,
                inner: vec![],
            }),
        }
    }
}

impl CustomValueSupport for NuSelector {
    type CV = NuSelectorCustomValue;

    fn custom_value(self) -> Self::CV {
        NuSelectorCustomValue {
            id: self.id,
            selector: Some(self),
        }
    }

    fn get_type_static() -> PolarsPluginType {
        PolarsPluginType::NuSelector
    }

    fn base_value(self, span: Span) -> Result<Value, ShellError> {
        self.to_value(span)
    }

    fn try_from_value(plugin: &PolarsPlugin, value: &Value) -> Result<Self, ShellError> {
        match value {
            Value::String { val, .. } => {
                let selector = Selector::ByName {
                    names: [val.into()].into(),
                    strict: true,
                };
                Ok(NuSelector::new(Some(selector)))
            }
            Value::List { vals, .. } => {
                let columns: Vec<PlSmallStr> = vals
                    .iter()
                    .map(|v| v.as_str().map(PlSmallStr::from_str))
                    .collect::<Result<Vec<PlSmallStr>, ShellError>>()?;
                let selector = Selector::ByName {
                    names: columns.into(),
                    strict: true,
                };
                Ok(NuSelector::new(Some(selector)))
            }
            Value::Custom {
                val, internal_span, ..
            } => {
                if let Some(cv) = val.as_any().downcast_ref::<Self::CV>() {
                    Self::try_from_custom_value(plugin, cv)
                } else if let Some(cv) = val.as_any().downcast_ref::<NuExpressionCustomValue>() {
                    let expr = NuExpression::try_from_custom_value(plugin, cv)?;
                    let selector = expr.into_polars().try_into_selector().map_err(|e| {
                        ShellError::GenericError {
                            error: e.to_string(),
                            msg: "".into(),
                            span: Some(*internal_span),
                            help: None,
                            inner: vec![],
                        }
                    })?;
                    Ok(NuSelector::new(Some(selector)))
                } else {
                    Err(ShellError::CantConvert {
                        to_type: Self::get_type_static().to_string(),
                        from_type: value.get_type().to_string(),
                        span: value.span(),
                        help: None,
                    })
                }
            }
            _ => Err(ShellError::CantConvert {
                to_type: Self::get_type_static().to_string(),
                from_type: value.get_type().to_string(),
                span: value.span(),
                help: None,
            }),
        }
    }
}
