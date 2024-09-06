use nu_protocol::{Span, Value};

use crate::{
    lazy_command,
    values::{Column, NuDataFrame},
};

// LazyReverse command
// Expands to a command definition for reverse
lazy_command!(
    LazyReverse,
    "polars reverse",
    "Reverses the LazyFrame",
    vec![Example {
        description: "Reverses the dataframe.",
        example: "[[a b]; [6 2] [4 2] [2 2]] | polars into-df | polars reverse",
        result: Some(
            NuDataFrame::try_from_columns(
                vec![
                    Column::new(
                        "a".to_string(),
                        vec![Value::test_int(2), Value::test_int(4), Value::test_int(6),],
                    ),
                    Column::new(
                        "b".to_string(),
                        vec![Value::test_int(2), Value::test_int(2), Value::test_int(2),],
                    ),
                ],
                None
            )
            .expect("simple df for test should not fail")
            .into_value(Span::test_data()),
        ),
    },],
    reverse,
    test_reverse
);
