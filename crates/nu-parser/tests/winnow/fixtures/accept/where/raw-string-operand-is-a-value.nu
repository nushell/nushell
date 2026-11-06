# A raw string in a row condition is a value, not a column of the row.
[{a: 1}] | where r#'a'# == 'a'
