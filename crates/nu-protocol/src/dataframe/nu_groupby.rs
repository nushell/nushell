use nu_source::{Span, Tag};
use polars::frame::groupby::{GroupBy, GroupTuples};
use serde::{Deserialize, Serialize};

use super::{FrameStruct, NuDataFrame};
use nu_errors::ShellError;

use crate::{TaggedDictBuilder, UntaggedValue, Value};

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub struct NuGroupBy {
    dataframe: NuDataFrame,
    by: Vec<String>,
    groups: GroupTuples,
}

impl NuGroupBy {
    pub fn new(dataframe: NuDataFrame, by: Vec<String>, groups: GroupTuples) -> Self {
        NuGroupBy {
            dataframe,
            by,
            groups,
        }
    }

    pub fn by(&self) -> &[String] {
        &self.by
    }

    pub fn try_from_stream<T>(input: &mut T, span: &Span) -> Result<NuGroupBy, ShellError>
    where
        T: Iterator<Item = Value>,
    {
        input
            .next()
            .and_then(|value| match value.value {
                UntaggedValue::FrameStruct(FrameStruct::GroupBy(group)) => Some(group),
                _ => None,
            })
            .ok_or_else(|| {
                ShellError::labeled_error(
                    "No groupby object in stream",
                    "no groupby object found in input stream",
                    span,
                )
            })
    }

    pub fn to_groupby(&self) -> Result<GroupBy, ShellError> {
        let df = self.dataframe.as_ref();

        let by = df.select_series(&self.by).map_err(|e| {
            ShellError::labeled_error("Error creating groupby", e.to_string(), Tag::unknown())
        })?;

        Ok(GroupBy::new(df, by, self.groups.clone(), None))
    }

    pub fn print(&self) -> Result<Vec<Value>, ShellError> {
        let mut values: Vec<Value> = Vec::new();

        let mut data = TaggedDictBuilder::new(Tag::unknown());
        data.insert_value("property", "group by");
        data.insert_value("value", self.by.join(", "));
        values.push(data.into_value());

        Ok(values)
    }
}

impl AsRef<polars::prelude::DataFrame> for NuGroupBy {
    fn as_ref(&self) -> &polars::prelude::DataFrame {
        self.dataframe.as_ref()
    }
}
