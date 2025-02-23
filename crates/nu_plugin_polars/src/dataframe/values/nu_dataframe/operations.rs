use nu_protocol::{ast::Operator, ShellError, Span, Spanned, Value};
use polars::prelude::{Column as PolarsColumn, DataFrame};

use crate::values::CustomValueSupport;
use crate::PolarsPlugin;

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
        plugin: &PolarsPlugin,
        lhs_span: Span,
        operator: Operator,
        op_span: Span,
        right: &Value,
    ) -> Result<NuDataFrame, ShellError> {
        let rhs_span = right.span();
        match right {
            Value::Custom { .. } => {
                let rhs = NuDataFrame::try_from_value_coerce(plugin, right, rhs_span)?;

                match (self.is_series(), rhs.is_series()) {
                    (true, true) => {
                        let lhs = &self
                            .as_series(lhs_span)
                            .expect("Already checked that is a series");
                        let rhs = &rhs
                            .as_series(rhs_span)
                            .expect("Already checked that is a series");

                        if lhs.dtype() != rhs.dtype() {
                            return Err(ShellError::IncompatibleParameters {
                                left_message: format!("datatype {}", lhs.dtype()),
                                left_span: lhs_span,
                                right_message: format!("datatype {}", lhs.dtype()),
                                right_span: rhs_span,
                            });
                        }

                        if lhs.len() != rhs.len() {
                            return Err(ShellError::IncompatibleParameters {
                                left_message: format!("len {}", lhs.len()),
                                left_span: lhs_span,
                                right_message: format!("len {}", rhs.len()),
                                right_span: rhs_span,
                            });
                        }

                        let op = Spanned {
                            item: operator,
                            span: op_span,
                        };

                        compute_between_series(
                            op,
                            &NuDataFrame::default().into_value(lhs_span),
                            lhs,
                            right,
                            rhs,
                        )
                    }
                    _ => {
                        if self.df.height() != rhs.df.height() {
                            return Err(ShellError::IncompatibleParameters {
                                left_message: format!("rows {}", self.df.height()),
                                left_span: lhs_span,
                                right_message: format!("rows {}", rhs.df.height()),
                                right_span: rhs_span,
                            });
                        }

                        let op = Spanned {
                            item: operator,
                            span: op_span,
                        };

                        between_dataframes(
                            op,
                            &NuDataFrame::default().into_value(lhs_span),
                            self,
                            right,
                            &rhs,
                        )
                    }
                }
            }
            _ => {
                let op = Spanned {
                    item: operator,
                    span: op_span,
                };

                compute_series_single_value(
                    op,
                    &NuDataFrame::default().into_value(lhs_span),
                    self,
                    right,
                )
            }
        }
    }

    pub fn append_df(
        &self,
        other: &NuDataFrame,
        axis: Axis,
        span: Span,
    ) -> Result<NuDataFrame, ShellError> {
        match axis {
            Axis::Row => {
                let mut columns: Vec<&str> = Vec::new();

                let new_cols = self
                    .df
                    .get_columns()
                    .iter()
                    .chain(other.df.get_columns())
                    .map(|s| {
                        let name = if columns.contains(&s.name().as_str()) {
                            format!("{}_{}", s.name(), "x")
                        } else {
                            columns.push(s.name());
                            s.name().to_string()
                        };

                        let mut series = s.clone();
                        series.rename(name.into());
                        series
                    })
                    .collect::<Vec<PolarsColumn>>();

                let df_new = DataFrame::new(new_cols).map_err(|e| ShellError::GenericError {
                    error: "Error creating dataframe".into(),
                    msg: e.to_string(),
                    span: Some(span),
                    help: None,
                    inner: vec![],
                })?;

                Ok(NuDataFrame::new(false, df_new))
            }
            Axis::Column => {
                if self.df.width() != other.df.width() {
                    return Err(ShellError::IncompatibleParametersSingle {
                        msg: "Dataframes with different number of columns".into(),
                        span,
                    });
                }

                if !self
                    .df
                    .get_column_names()
                    .iter()
                    .all(|col| other.df.get_column_names().contains(col))
                {
                    return Err(ShellError::IncompatibleParametersSingle {
                        msg: "Dataframes with different columns names".into(),
                        span,
                    });
                }

                let new_cols = self
                    .df
                    .get_columns()
                    .iter()
                    .map(|s| {
                        let other_col = other
                            .df
                            .column(s.name())
                            .expect("Already checked that dataframes have same columns");

                        let mut tmp = s.clone();
                        let res = tmp.append(other_col);

                        match res {
                            Ok(s) => Ok(s.clone()),
                            Err(e) => Err({
                                ShellError::GenericError {
                                    error: "Error appending dataframe".into(),
                                    msg: format!("Unable to append: {e}"),
                                    span: Some(span),
                                    help: None,
                                    inner: vec![],
                                }
                            }),
                        }
                    })
                    .collect::<Result<Vec<PolarsColumn>, ShellError>>()?;

                let df_new = DataFrame::new(new_cols).map_err(|e| ShellError::GenericError {
                    error: "Error appending dataframe".into(),
                    msg: format!("Unable to append dataframes: {e}"),
                    span: Some(span),
                    help: None,
                    inner: vec![],
                })?;

                Ok(NuDataFrame::new(false, df_new))
            }
        }
    }
}
