use nu_protocol::{ast::Operator, ShellError, Span, Spanned, Value};
use polars::prelude::{DataFrame, Series};

use super::between_values::{
    between_dataframes, compute_between_series, compute_series_single_value,
};

use super::NuDataFrame;

pub enum Axis {
    Row,
    Column,
}

impl NuDataFrame {
    pub fn compute_with_value(
        &self,
        lhs_span: Span,
        operator: Operator,
        op_span: Span,
        right: &Value,
    ) -> Result<Value, ShellError> {
        match right {
            Value::CustomValue {
                val: rhs,
                span: rhs_span,
            } => {
                let rhs = rhs.as_any().downcast_ref::<NuDataFrame>().ok_or_else(|| {
                    ShellError::DowncastNotPossible(
                        "Unable to create dataframe".to_string(),
                        *rhs_span,
                    )
                })?;

                match (self.is_series(), rhs.is_series()) {
                    (true, true) => {
                        let lhs = &self
                            .as_series(lhs_span)
                            .expect("Already checked that is a series");
                        let rhs = &rhs
                            .as_series(*rhs_span)
                            .expect("Already checked that is a series");

                        if lhs.dtype() != rhs.dtype() {
                            return Err(ShellError::IncompatibleParameters {
                                left_message: format!("datatype {}", lhs.dtype()),
                                left_span: lhs_span,
                                right_message: format!("datatype {}", lhs.dtype()),
                                right_span: *rhs_span,
                            });
                        }

                        if lhs.len() != rhs.len() {
                            return Err(ShellError::IncompatibleParameters {
                                left_message: format!("len {}", lhs.len()),
                                left_span: lhs_span,
                                right_message: format!("len {}", rhs.len()),
                                right_span: *rhs_span,
                            });
                        }

                        let op = Spanned {
                            item: operator,
                            span: op_span,
                        };

                        compute_between_series(
                            op,
                            &NuDataFrame::default_value(lhs_span),
                            lhs,
                            right,
                            rhs,
                        )
                    }
                    _ => {
                        if self.0.height() != rhs.0.height() {
                            return Err(ShellError::IncompatibleParameters {
                                left_message: format!("rows {}", self.0.height()),
                                left_span: lhs_span,
                                right_message: format!("rows {}", rhs.0.height()),
                                right_span: *rhs_span,
                            });
                        }

                        let op = Spanned {
                            item: operator,
                            span: op_span,
                        };

                        between_dataframes(
                            op,
                            &NuDataFrame::default_value(lhs_span),
                            self,
                            right,
                            rhs,
                        )
                    }
                }
            }
            _ => {
                let op = Spanned {
                    item: operator,
                    span: op_span,
                };

                compute_series_single_value(op, &NuDataFrame::default_value(lhs_span), self, right)
            }
        }
    }

    pub fn append_df(
        &self,
        other: &NuDataFrame,
        axis: Axis,
        span: Span,
    ) -> Result<Self, ShellError> {
        match axis {
            Axis::Row => {
                let mut columns: Vec<&str> = Vec::new();

                let new_cols = self
                    .0
                    .get_columns()
                    .iter()
                    .chain(other.0.get_columns())
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
                    ShellError::SpannedLabeledError(
                        "Error creating dataframe".into(),
                        e.to_string(),
                        span,
                    )
                })?;

                Ok(NuDataFrame::new(df_new))
            }
            Axis::Column => {
                if self.0.width() != other.0.width() {
                    return Err(ShellError::IncompatibleParametersSingle(
                        "Dataframes with different number of columns".into(),
                        span,
                    ));
                }

                if !self
                    .0
                    .get_column_names()
                    .iter()
                    .all(|col| other.0.get_column_names().contains(col))
                {
                    return Err(ShellError::IncompatibleParametersSingle(
                        "Dataframes with different columns names".into(),
                        span,
                    ));
                }

                let new_cols = self
                    .0
                    .get_columns()
                    .iter()
                    .map(|s| {
                        let other_col = other
                            .0
                            .column(s.name())
                            .expect("Already checked that dataframes have same columns");

                        let mut tmp = s.clone();
                        let res = tmp.append(other_col);

                        match res {
                            Ok(s) => Ok(s.clone()),
                            Err(e) => Err({
                                ShellError::SpannedLabeledError(
                                    "Error appending dataframe".into(),
                                    format!("Unable to append: {}", e),
                                    span,
                                )
                            }),
                        }
                    })
                    .collect::<Result<Vec<Series>, ShellError>>()?;

                let df_new = DataFrame::new(new_cols).map_err(|e| {
                    ShellError::SpannedLabeledError(
                        "Error appending dataframe".into(),
                        format!("Unable to append dataframes: {}", e.to_string()),
                        span,
                    )
                })?;

                Ok(NuDataFrame::new(df_new))
            }
        }
    }
}
