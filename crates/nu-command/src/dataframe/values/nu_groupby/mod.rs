mod custom_value;

use nu_protocol::{PipelineData, ShellError, Span, Value};
use polars::frame::groupby::{GroupBy, GroupTuples};
use polars::prelude::DataFrame;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct NuGroupBy {
    dataframe: DataFrame,
    by: Vec<String>,
    groups: GroupTuples,
}

impl NuGroupBy {
    pub fn new(dataframe: DataFrame, by: Vec<String>, groups: GroupTuples) -> Self {
        NuGroupBy {
            dataframe,
            by,
            groups,
        }
    }

    pub fn into_value(self, span: Span) -> Value {
        Value::CustomValue {
            val: Box::new(self),
            span,
        }
    }

    pub fn try_from_value(value: Value) -> Result<Self, ShellError> {
        match value {
            Value::CustomValue { val, span } => match val.as_any().downcast_ref::<NuGroupBy>() {
                Some(groupby) => Ok(NuGroupBy {
                    dataframe: groupby.dataframe.clone(),
                    by: groupby.by.clone(),
                    groups: groupby.groups.clone(),
                }),
                None => Err(ShellError::CantConvert(
                    "groupby".into(),
                    "non-dataframe".into(),
                    span,
                )),
            },
            x => Err(ShellError::CantConvert(
                "groupby".into(),
                x.get_type().to_string(),
                x.span()?,
            )),
        }
    }

    pub fn try_from_pipeline(input: PipelineData, span: Span) -> Result<NuGroupBy, ShellError> {
        let value = input.into_value(span);
        NuGroupBy::try_from_value(value)
    }

    pub fn to_groupby(&self) -> Result<GroupBy, ShellError> {
        let by = self.dataframe.select_series(&self.by).map_err(|e| {
            ShellError::LabeledError("Error creating groupby".into(), e.to_string())
        })?;

        Ok(GroupBy::new(&self.dataframe, by, self.groups.clone(), None))
    }

    pub fn print(&self, span: Span) -> Result<Vec<Value>, ShellError> {
        let values = self
            .by
            .iter()
            .map(|col| {
                let cols = vec!["group by".to_string()];
                let vals = vec![Value::String {
                    val: col.into(),
                    span,
                }];

                Value::Record { cols, vals, span }
            })
            .collect::<Vec<Value>>();

        Ok(values)
    }
}

impl AsRef<DataFrame> for NuGroupBy {
    fn as_ref(&self) -> &polars::prelude::DataFrame {
        &self.dataframe
    }
}
