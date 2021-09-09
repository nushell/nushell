use nu_errors::ShellError;
use nu_source::Span;
use polars::prelude::{DataFrame, Series};

use super::NuDataFrame;

pub enum Axis {
    Row,
    Column,
}

impl Axis {
    pub fn try_from_str(axis: &str, span: &Span) -> Result<Axis, ShellError> {
        match axis {
            "row" => Ok(Axis::Row),
            "col" => Ok(Axis::Column),
            _ => Err(ShellError::labeled_error_with_secondary(
                "Wrong axis",
                "The selected axis does not exist",
                span,
                "The only axis options are 'row' or 'col'",
                span,
            )),
        }
    }
}

impl NuDataFrame {
    pub fn append_df(
        &self,
        other: &NuDataFrame,
        axis: Axis,
        span: &Span,
    ) -> Result<Self, ShellError> {
        match axis {
            Axis::Row => {
                let mut columns: Vec<&str> = Vec::new();

                let new_cols = self
                    .as_ref()
                    .get_columns()
                    .iter()
                    .chain(other.as_ref().get_columns())
                    .map(|s| {
                        let name = if columns.contains(&s.name()) {
                            format!("{}_{}", s.name(), "x")
                        } else {
                            columns.push(s.name());
                            s.name().to_string()
                        };

                        let mut series = s.clone();
                        series.rename(&name);
                        series
                    })
                    .collect::<Vec<Series>>();

                let df_new = DataFrame::new(new_cols).map_err(|e| {
                    ShellError::labeled_error("Appending error", e.to_string(), span)
                })?;

                Ok(NuDataFrame::new(df_new))
            }
            Axis::Column => {
                if self.as_ref().width() != other.as_ref().width() {
                    return Err(ShellError::labeled_error(
                        "Appending error",
                        "Dataframes with different number of columns",
                        span,
                    ));
                }

                if !self
                    .as_ref()
                    .get_column_names()
                    .iter()
                    .all(|col| other.as_ref().get_column_names().contains(col))
                {
                    return Err(ShellError::labeled_error(
                        "Appending error",
                        "Dataframes with different columns names",
                        span,
                    ));
                }

                let new_cols = self
                    .as_ref()
                    .get_columns()
                    .iter()
                    .map(|s| {
                        let other_col = other
                            .as_ref()
                            .column(s.name())
                            .expect("Already checked that dataframes have same columns");

                        let mut tmp = s.clone();
                        let res = tmp.append(other_col);

                        match res {
                            Ok(s) => Ok(s.clone()),
                            Err(e) => Err({
                                ShellError::labeled_error(
                                    "Appending error",
                                    format!("Unable to append dataframes: {}", e),
                                    span,
                                )
                            }),
                        }
                    })
                    .collect::<Result<Vec<Series>, ShellError>>()?;

                let df_new = DataFrame::new(new_cols).map_err(|e| {
                    ShellError::labeled_error("Appending error", e.to_string(), span)
                })?;

                Ok(NuDataFrame::new(df_new))
            }
        }
    }
}
