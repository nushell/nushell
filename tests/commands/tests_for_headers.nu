use std assert


# This is the custom command 1 for headers:

#[test]
def headers_sets_the_column_names_for_a_table_created_by_split_column_1 [] {
  let result = ("a b c|1 2 3" | split row "|" | split column " " | headers)
  assert ($result == [{a: 1, b: 2, c: 3}])
}

# This is the custom command 2 for headers:

#[test]
def headers_columns_which_dont_have_data_in_their_first_row_are_removed_2 [] {
  let result = ("a b c|1 2 3|1 2 3 4" | split row "|" | split column " " | headers)
  assert ($result == [{a: 1, b: 2, c: 3}, {a: 1, b: 2, c: 3}])
}


