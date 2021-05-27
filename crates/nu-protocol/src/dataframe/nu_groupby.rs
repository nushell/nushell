use nu_source::Tag;
use polars::frame::groupby::{GroupBy, GroupTuples};
use serde::{Deserialize, Serialize};

use super::NuDataFrame;
use nu_errors::ShellError;

use crate::{TaggedDictBuilder, Value};

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

    pub fn to_groupby(&self) -> Result<GroupBy, ShellError> {
        let df = match &self.dataframe.dataframe {
            Some(df) => df,
            None => unreachable!("No dataframe in nu_dataframe"),
        };

        let by = df.select_series(&self.by).map_err(|e| {
            ShellError::labeled_error("Error creating groupby", format!("{}", e), Tag::unknown())
        })?;

        Ok(GroupBy::new(df, by, self.groups.clone(), None))
    }

    pub fn print(&self) -> Result<Vec<Value>, ShellError> {
        let mut values: Vec<Value> = Vec::new();

        let mut data = TaggedDictBuilder::new(Tag::unknown());
        data.insert_value("property", "dataframe");
        data.insert_value("value", self.dataframe.name.as_ref());
        values.push(data.into_value());

        let mut data = TaggedDictBuilder::new(Tag::unknown());
        data.insert_value("property", "group by");
        data.insert_value("value", self.by.join(", "));
        values.push(data.into_value());

        Ok(values)
    }

    pub fn dataframe_ref(&self) -> &polars::prelude::DataFrame {
        match &self.dataframe.dataframe {
            Some(df) => df,
            None => unreachable!("Accessing reference to dataframe from groupby"),
        }
    }
}
