use std assert


# This is the custom command 1 for reject:

#[test]
def reject_reject_a_column_in_the_ls_table_1 [] {
  let result = (ls | reject modified)
  assert ($result == )
}

# This is the custom command 2 for reject:

#[test]
def reject_reject_a_column_in_a_table_2 [] {
  let result = ([[a, b]; [1, 2]] | reject a)
  assert ($result == [{b: 2}])
}

# This is the custom command 3 for reject:

#[test]
def reject_reject_a_row_in_a_table_3 [] {
  let result = ([[a, b]; [1, 2] [3, 4]] | reject 1)
  assert ($result == [{a: 1, b: 2}])
}

# This is the custom command 4 for reject:

#[test]
def reject_reject_the_specified_field_in_a_record_4 [] {
  let result = ({a: 1, b: 2} | reject a)
  assert ($result == {b: 2})
}

# This is the custom command 5 for reject:

#[test]
def reject_reject_a_nested_field_in_a_record_5 [] {
  let result = ({a: {b: 3, c: 5}} | reject a.b)
  assert ($result == {a: {c: 5}})
}

# This is the custom command 6 for reject:

#[test]
def reject_reject_columns_by_a_provided_list_of_columns_6 [] {
  let result = (let cols = [size type];[[name type size]; [Cargo.toml toml 1kb] [Cargo.lock toml 2kb]] | reject $cols)
  assert ($result == )
}

# This is the custom command 7 for reject:

#[test]
def reject_reject_rows_by_a_provided_list_of_rows_7 [] {
  let result = (let rows = [0 2];[[name type size]; [Cargo.toml toml 1kb] [Cargo.lock toml 2kb] [file.json json 3kb]] | reject $rows)
  assert ($result == )
}


