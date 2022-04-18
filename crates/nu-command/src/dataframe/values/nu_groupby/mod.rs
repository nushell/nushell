mod custom_value;

use nu_protocol::{PipelineData, ShellError, Span, Value};
use polars::frame::groupby::{GroupBy, GroupsProxy};
use polars::prelude::{DataFrame, GroupsIdx};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NuGroupsProxy {
    Idx {
        sorted: bool,
        all: Vec<(u32, Vec<u32>)>,
    },
    Slice(Vec<[u32; 2]>),
}

impl NuGroupsProxy {
    fn from_polars(groups: &GroupsProxy) -> Self {
        match groups {
            GroupsProxy::Idx(indexes) => NuGroupsProxy::Idx {
                sorted: indexes.is_sorted(),
                all: indexes
                    .iter()
                    .map(|(index, values)| (index, values.clone()))
                    .collect(),
            },
            GroupsProxy::Slice(slice) => NuGroupsProxy::Slice(slice.clone()),
        }
    }

    fn to_polars(&self) -> GroupsProxy {
        match self {
            Self::Idx { sorted, all } => {
                let mut groups: GroupsIdx = all.clone().into();
                if *sorted {
                    groups.sort()
                }

                GroupsProxy::Idx(groups)
            }
            Self::Slice(slice) => GroupsProxy::Slice(slice.clone()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NuGroupBy {
    dataframe: DataFrame,
    by: Vec<String>,
    groups: NuGroupsProxy,
}

impl NuGroupBy {
    pub fn new(dataframe: DataFrame, by: Vec<String>, groups: &GroupsProxy) -> Self {
        NuGroupBy {
            dataframe,
            by,
            groups: NuGroupsProxy::from_polars(groups),
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
                    None,
                )),
            },
            x => Err(ShellError::CantConvert(
                "groupby".into(),
                x.get_type().to_string(),
                x.span()?,
                None,
            )),
        }
    }

    pub fn try_from_pipeline(input: PipelineData, span: Span) -> Result<NuGroupBy, ShellError> {
        let value = input.into_value(span);
        NuGroupBy::try_from_value(value)
    }

    pub fn to_groupby(&self) -> Result<GroupBy, ShellError> {
        let by = self.dataframe.select_series(&self.by).map_err(|e| {
            ShellError::GenericError(
                "Error creating groupby".into(),
                "".to_string(),
                None,
                Some(e.to_string()),
                Vec::new(),
            )
        })?;

        Ok(GroupBy::new(
            &self.dataframe,
            by,
            self.groups.to_polars(),
            None,
        ))
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
