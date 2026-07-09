use ndarray::ArrayD;
use nu_protocol::{
    CellPathMutation, CustomValue, ShellError, Span, Type, Value,
    ast::{Comparison, Math, Operator, PathMember},
    casing::Casing,
};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::cmp::Ordering;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixValue {
    pub array: ArrayD<f64>,
}

#[typetag::serde]
impl CustomValue for MatrixValue {
    fn clone_value(&self, span: Span) -> Value {
        Value::custom(Box::new(self.clone()), span)
    }

    fn type_name(&self) -> String {
        "matrix".to_string()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        Ok(ndarray_to_value(&self.array, span))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }

    fn partial_cmp(&self, other: &Value) -> Option<Ordering> {
        match other {
            Value::Custom { val, .. } if val.type_name() == self.type_name() => {
                let other_matrix = val.as_any().downcast_ref::<MatrixValue>()?;
                if self.array.shape() != other_matrix.array.shape() {
                    return None;
                }
                if ndarray::Zip::from(&self.array)
                    .and(&other_matrix.array)
                    .all(|a, b| a == b)
                {
                    Some(Ordering::Equal)
                } else if ndarray::Zip::from(&self.array)
                    .and(&other_matrix.array)
                    .all(|a, b| *a <= *b)
                {
                    Some(Ordering::Less)
                } else if ndarray::Zip::from(&self.array)
                    .and(&other_matrix.array)
                    .all(|a, b| *a >= *b)
                {
                    Some(Ordering::Greater)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn follow_path_string(
        &self,
        self_span: Span,
        column_name: String,
        path_span: Span,
        _optional: bool,
        casing: Casing,
    ) -> Result<Value, ShellError> {
        let col = match casing {
            Casing::Sensitive => column_name,
            Casing::Insensitive => column_name.to_lowercase(),
        };

        match col.as_str() {
            "shape" => Ok(Value::list(
                self.array
                    .shape()
                    .iter()
                    .map(|d| Value::int(*d as i64, path_span))
                    .collect(),
                path_span,
            )),
            "ndim" => Ok(Value::int(self.array.ndim() as i64, path_span)),
            "size" => Ok(Value::int(self.array.len() as i64, path_span)),
            _ => Err(ShellError::CantFindColumn {
                col_name: col,
                span: Some(path_span),
                src_span: self_span,
            }),
        }
    }

    fn follow_path_int(
        &self,
        self_span: Span,
        index: usize,
        path_span: Span,
        _optional: bool,
    ) -> Result<Value, ShellError> {
        if self.array.ndim() == 0 {
            return Err(ShellError::IncompatiblePathAccess {
                type_name: self.type_name(),
                span: path_span,
            });
        }
        if index >= self.array.shape()[0] {
            return Err(ShellError::AccessBeyondEnd {
                max_idx: self.array.shape()[0] - 1,
                span: self_span,
            });
        }
        let subview = self.array.index_axis(ndarray::Axis(0), index);
        if subview.ndim() == 0 {
            Ok(Value::float(*subview.first().unwrap_or(&0.0), path_span))
        } else {
            Ok(ndarray_to_value(&subview.to_owned(), path_span))
        }
    }

    fn update_data_at_cell_path(
        &self,
        cell_path: &[PathMember],
        new_val: Value,
        action: &CellPathMutation,
        head: Span,
    ) -> Result<Value, ShellError> {
        let mut base = self.to_base_value(head)?;
        base.mutate_data_at_cell_path(cell_path, new_val, action)?;
        match base {
            Value::List { vals, .. } => {
                MatrixValue::from_list_of_lists(&vals, head).map(|m| m.into_value(head))
            }
            other => Ok(other),
        }
    }

    fn operation(
        &self,
        lhs_span: Span,
        operator: Operator,
        op: Span,
        right: &Value,
    ) -> Result<Value, ShellError> {
        match operator {
            Operator::Math(Math::Add) => matrix_math_op(self, right, op, lhs_span, |a, b| a + b),
            Operator::Math(Math::Subtract) => {
                matrix_math_op(self, right, op, lhs_span, |a, b| a - b)
            }
            Operator::Math(Math::Multiply) => {
                matrix_math_op(self, right, op, lhs_span, |a, b| a * b)
            }
            Operator::Math(Math::Divide) => {
                matrix_scalar_op(self, right, op, lhs_span, |a, s| a / s)
            }
            Operator::Comparison(Comparison::Equal) => {
                compare_matrix(self, right, op, lhs_span, Ordering::Equal)
            }
            Operator::Comparison(Comparison::NotEqual) => {
                compare_matrix(self, right, op, lhs_span, Ordering::Equal)
                    .map(|v| Value::bool(!matches!(v, Value::Bool { val: true, .. }), op))
            }
            Operator::Comparison(Comparison::LessThan) => {
                compare_matrix(self, right, op, lhs_span, Ordering::Less)
            }
            Operator::Comparison(Comparison::GreaterThan) => {
                compare_matrix(self, right, op, lhs_span, Ordering::Greater)
            }
            Operator::Comparison(Comparison::LessThanOrEqual) => {
                let result: Option<bool> =
                    compare_matrix(self, right, op, lhs_span, Ordering::Less)
                        .ok()
                        .and_then(|v| {
                            if matches!(v, Value::Bool { val: true, .. }) {
                                Some(true)
                            } else if matches!(v, Value::Bool { val: false, .. }) {
                                Some(false)
                            } else {
                                None
                            }
                        });
                let equal = compare_matrix(self, right, op, lhs_span, Ordering::Equal)
                    .ok()
                    .and_then(|v| {
                        if let Value::Bool { val: b, .. } = v {
                            Some(b)
                        } else {
                            None
                        }
                    });
                Ok(Value::bool(
                    result.unwrap_or(false) || equal.unwrap_or(false),
                    op,
                ))
            }
            Operator::Comparison(Comparison::GreaterThanOrEqual) => {
                let result: Option<bool> =
                    compare_matrix(self, right, op, lhs_span, Ordering::Greater)
                        .ok()
                        .and_then(|v| {
                            if matches!(v, Value::Bool { val: true, .. }) {
                                Some(true)
                            } else if matches!(v, Value::Bool { val: false, .. }) {
                                Some(false)
                            } else {
                                None
                            }
                        });
                let equal = compare_matrix(self, right, op, lhs_span, Ordering::Equal)
                    .ok()
                    .and_then(|v| {
                        if let Value::Bool { val: b, .. } = v {
                            Some(b)
                        } else {
                            None
                        }
                    });
                Ok(Value::bool(
                    result.unwrap_or(false) || equal.unwrap_or(false),
                    op,
                ))
            }
            _ => Err(ShellError::OperatorUnsupportedType {
                op: operator,
                unsupported: Type::Custom(self.type_name().into()),
                op_span: op,
                unsupported_span: lhs_span,
                help: None,
            }),
        }
    }
}

impl MatrixValue {
    pub fn new(array: ArrayD<f64>) -> Self {
        Self { array }
    }

    pub fn into_value(self, span: Span) -> Value {
        Value::custom(Box::new(self), span)
    }

    pub fn from_value(value: &Value) -> Result<Self, ShellError> {
        let span = value.span();
        match value {
            Value::Custom { val, .. } => {
                val.as_any().downcast_ref::<Self>().cloned().ok_or_else(|| {
                    ShellError::CantConvert {
                        to_type: "matrix".into(),
                        from_type: val.type_name(),
                        span,
                        help: Some("expected a matrix value".into()),
                    }
                })
            }
            x => Err(ShellError::CantConvert {
                to_type: "matrix".into(),
                from_type: x.get_type().to_string(),
                span,
                help: None,
            }),
        }
    }

    pub fn from_shape_vec(
        shape: Vec<usize>,
        data: Vec<f64>,
        span: Span,
    ) -> Result<Self, ShellError> {
        ArrayD::from_shape_vec(shape, data)
            .map(Self::new)
            .map_err(|e| {
                ShellError::Generic(nu_protocol::shell_error::generic::GenericError::new(
                    "Matrix shape error",
                    e.to_string(),
                    span,
                ))
            })
    }

    pub fn from_list_of_lists(values: &[Value], span: Span) -> Result<Self, ShellError> {
        let mut rows: Vec<Vec<f64>> = Vec::new();
        let mut ncols: Option<usize> = None;

        for (i, value) in values.iter().enumerate() {
            match value {
                Value::List { vals, .. } => {
                    let row: Result<Vec<f64>, ShellError> =
                        vals.iter().map(|v| value_to_f64(v, span)).collect();
                    let row = row?;
                    if let Some(expected) = ncols {
                        if row.len() != expected {
                            return Err(ShellError::Generic(
                                nu_protocol::shell_error::generic::GenericError::new(
                                    "Inconsistent row lengths",
                                    format!(
                                        "row {} has {} elements, expected {}",
                                        i,
                                        row.len(),
                                        expected
                                    ),
                                    span,
                                ),
                            ));
                        }
                    } else {
                        ncols = Some(row.len());
                    }
                    rows.push(row);
                }
                _ => {
                    return Err(ShellError::Generic(
                        nu_protocol::shell_error::generic::GenericError::new(
                            "Invalid matrix input",
                            format!("row {} is not a list", i),
                            span,
                        ),
                    ));
                }
            }
        }

        let nrows = rows.len();
        let ncols = ncols.unwrap_or(0);
        let flat: Vec<f64> = rows.into_iter().flatten().collect();

        Self::from_shape_vec(vec![nrows, ncols], flat, span)
    }

    pub fn from_list_of_records(values: &[Value], span: Span) -> Result<Self, ShellError> {
        if values.is_empty() {
            let array = ArrayD::from_shape_vec(vec![0, 0], vec![]).map_err(|e| {
                ShellError::Generic(nu_protocol::shell_error::generic::GenericError::new(
                    "Matrix shape error",
                    e.to_string(),
                    span,
                ))
            })?;
            return Ok(Self::new(array));
        }

        let first_record = match &values[0] {
            Value::Record { val, .. } => val,
            _ => {
                return Err(ShellError::Generic(
                    nu_protocol::shell_error::generic::GenericError::new(
                        "Invalid matrix input",
                        "expected a list of records",
                        span,
                    ),
                ));
            }
        };

        let cols: Vec<String> = first_record.columns().cloned().collect();
        let ncols = cols.len();
        let nrows = values.len();

        let mut data = Vec::with_capacity(nrows * ncols);

        for (i, value) in values.iter().enumerate() {
            match value {
                Value::Record { val, .. } => {
                    for col in &cols {
                        let element = val.get(col).ok_or_else(|| {
                            ShellError::Generic(
                                nu_protocol::shell_error::generic::GenericError::new(
                                    "Missing column",
                                    format!("row {} is missing column '{}'", i, col),
                                    span,
                                ),
                            )
                        })?;
                        data.push(value_to_f64(element, span)?);
                    }
                }
                _ => {
                    return Err(ShellError::Generic(
                        nu_protocol::shell_error::generic::GenericError::new(
                            "Invalid matrix input",
                            format!("row {} is not a record", i),
                            span,
                        ),
                    ));
                }
            }
        }

        Self::from_shape_vec(vec![nrows, ncols], data, span)
    }

    pub fn test_value(rows: &[&[f64]]) -> Value {
        let nrows = rows.len();
        let ncols = if nrows > 0 { rows[0].len() } else { 0 };
        let flat: Vec<f64> = rows.iter().flat_map(|r| r.iter()).copied().collect();
        let array = ArrayD::from_shape_vec(vec![nrows, ncols], flat)
            .expect("test value shape must be valid");
        Value::test_custom_value(Box::new(Self { array }))
    }
}

fn value_to_f64(value: &Value, span: Span) -> Result<f64, ShellError> {
    match value {
        Value::Int { val, .. } => Ok(*val as f64),
        Value::Float { val, .. } => Ok(*val),
        Value::String { val, .. } => val.parse::<f64>().map_err(|_| ShellError::CantConvert {
            to_type: "float".into(),
            from_type: "string".into(),
            span,
            help: None,
        }),
        _ => Err(ShellError::CantConvert {
            to_type: "float".into(),
            from_type: value.get_type().to_string(),
            span,
            help: None,
        }),
    }
}

fn ndarray_to_value(array: &ArrayD<f64>, span: Span) -> Value {
    if array.ndim() == 0 {
        return Value::float(array.first().copied().unwrap_or(0.0), span);
    }

    if array.ndim() == 1 {
        let list: Vec<Value> = array.iter().map(|v| Value::float(*v, span)).collect();
        return Value::list(list, span);
    }

    if array.ndim() == 2 {
        let rows: Vec<Value> = array
            .axis_iter(ndarray::Axis(0))
            .map(|row| {
                let vals: Vec<Value> = row.iter().map(|v| Value::float(*v, span)).collect();
                Value::list(vals, span)
            })
            .collect();
        return Value::list(rows, span);
    }

    let sub_results: Vec<Value> = array
        .axis_iter(ndarray::Axis(0))
        .map(|sub| ndarray_to_value(&sub.to_owned(), span))
        .collect();
    Value::list(sub_results, span)
}

fn matrix_math_op<F>(
    left: &MatrixValue,
    right: &Value,
    op_span: Span,
    lhs_span: Span,
    f: F,
) -> Result<Value, ShellError>
where
    F: Fn(f64, f64) -> f64,
{
    match right {
        Value::Int { val, .. } => {
            let result = left.array.map(|v| f(*v, *val as f64));
            Ok(MatrixValue::new(result).into_value(op_span))
        }
        Value::Float { val, .. } => {
            let result = left.array.map(|v| f(*v, *val));
            Ok(MatrixValue::new(result).into_value(op_span))
        }
        Value::Custom { val, .. } => {
            let other = val.as_any().downcast_ref::<MatrixValue>().ok_or_else(|| {
                ShellError::OperatorIncompatibleTypes {
                    op: Operator::Math(Math::Add),
                    lhs: Type::Custom("matrix".into()),
                    rhs: Type::Custom(val.type_name().into()),
                    op_span,
                    lhs_span,
                    rhs_span: right.span(),
                    help: None,
                }
            })?;
            if left.array.shape() != other.array.shape() {
                return Err(ShellError::OperatorIncompatibleTypes {
                    op: Operator::Math(Math::Add),
                    lhs: Type::Custom("matrix".into()),
                    rhs: Type::Custom("matrix".into()),
                    op_span,
                    lhs_span,
                    rhs_span: right.span(),
                    help: None,
                });
            }
            let shape: Vec<usize> = left.array.shape().to_vec();
            let mut result = ArrayD::zeros(shape.clone());
            ndarray::Zip::from(&mut result)
                .and(&left.array)
                .and(&other.array)
                .for_each(|r, &a, &b| *r = f(a, b));
            Ok(MatrixValue::new(result).into_value(op_span))
        }
        _ => Err(ShellError::OperatorIncompatibleTypes {
            op: Operator::Math(Math::Add),
            lhs: Type::Custom("matrix".into()),
            rhs: right.get_type(),
            op_span,
            lhs_span,
            rhs_span: right.span(),
            help: Some("expected a matrix or scalar"),
        }),
    }
}

fn matrix_scalar_op<F>(
    left: &MatrixValue,
    right: &Value,
    op_span: Span,
    lhs_span: Span,
    f: F,
) -> Result<Value, ShellError>
where
    F: Fn(f64, f64) -> f64,
{
    match right {
        Value::Int { val, .. } => {
            let result = left.array.map(|v| f(*v, *val as f64));
            Ok(MatrixValue::new(result).into_value(op_span))
        }
        Value::Float { val, .. } => {
            let result = left.array.map(|v| f(*v, *val));
            Ok(MatrixValue::new(result).into_value(op_span))
        }
        Value::Custom { val, .. } => {
            if let Some(other) = val.as_any().downcast_ref::<MatrixValue>() {
                if left.array.shape() != other.array.shape() {
                    return Err(ShellError::OperatorIncompatibleTypes {
                        op: Operator::Math(Math::Divide),
                        lhs: Type::Custom("matrix".into()),
                        rhs: Type::Custom("matrix".into()),
                        op_span,
                        lhs_span,
                        rhs_span: right.span(),
                        help: Some("element-wise division on matrices requires matching shapes"),
                    });
                }
                let shape: Vec<usize> = left.array.shape().to_vec();
                let mut result = ArrayD::zeros(shape.clone());
                ndarray::Zip::from(&mut result)
                    .and(&left.array)
                    .and(&other.array)
                    .for_each(|r, &a, &b| *r = f(a, b));
                return Ok(MatrixValue::new(result).into_value(op_span));
            }
            Err(ShellError::OperatorIncompatibleTypes {
                op: Operator::Math(Math::Divide),
                lhs: Type::Custom("matrix".into()),
                rhs: Type::Custom(val.type_name().into()),
                op_span,
                lhs_span,
                rhs_span: right.span(),
                help: Some("expected a matrix or scalar"),
            })
        }
        _ => Err(ShellError::OperatorIncompatibleTypes {
            op: Operator::Math(Math::Divide),
            lhs: Type::Custom("matrix".into()),
            rhs: right.get_type(),
            op_span,
            lhs_span,
            rhs_span: right.span(),
            help: Some("expected a matrix or scalar"),
        }),
    }
}

fn compare_matrix(
    left: &MatrixValue,
    right: &Value,
    op_span: Span,
    lhs_span: Span,
    ordering: Ordering,
) -> Result<Value, ShellError> {
    match right {
        Value::Custom { val, .. } if val.type_name() == "matrix" => {
            let other = val.as_any().downcast_ref::<MatrixValue>().ok_or_else(|| {
                ShellError::OperatorIncompatibleTypes {
                    op: Operator::Comparison(Comparison::Equal),
                    lhs: Type::Custom("matrix".into()),
                    rhs: Type::Custom(val.type_name().into()),
                    op_span,
                    lhs_span,
                    rhs_span: right.span(),
                    help: None,
                }
            })?;

            if left.array.shape() != other.array.shape() {
                return Ok(Value::bool(ordering != Ordering::Equal, op_span));
            }

            let all_match = match ordering {
                Ordering::Equal => ndarray::Zip::from(&left.array)
                    .and(&other.array)
                    .all(|a, b| a == b),
                Ordering::Less => ndarray::Zip::from(&left.array)
                    .and(&other.array)
                    .all(|a, b| a < b),
                Ordering::Greater => ndarray::Zip::from(&left.array)
                    .and(&other.array)
                    .all(|a, b| a > b),
            };

            Ok(Value::bool(all_match, op_span))
        }
        Value::Int { val, .. } => {
            let s = *val as f64;
            let all_match = match ordering {
                Ordering::Equal => left.array.iter().all(|v| (*v - s).abs() < f64::EPSILON),
                Ordering::Less => left.array.iter().all(|&v| v < s),
                Ordering::Greater => left.array.iter().all(|&v| v > s),
            };
            Ok(Value::bool(all_match, op_span))
        }
        Value::Float { val, .. } => {
            let all_match = match ordering {
                Ordering::Equal => left.array.iter().all(|v| (*v - *val).abs() < f64::EPSILON),
                Ordering::Less => left.array.iter().all(|&v| v < *val),
                Ordering::Greater => left.array.iter().all(|&v| v > *val),
            };
            Ok(Value::bool(all_match, op_span))
        }
        _ => Err(ShellError::OperatorIncompatibleTypes {
            op: Operator::Comparison(Comparison::Equal),
            lhs: Type::Custom("matrix".into()),
            rhs: right.get_type(),
            op_span,
            lhs_span,
            rhs_span: right.span(),
            help: Some("expected a matrix or numeric scalar"),
        }),
    }
}
