use std assert

# Parameter name:
# sig type   : record
# name       : value
# type       : positional
# shape      : any
# description: the new value to merge with

# Parameter name:
# sig type   : table
# name       : value
# type       : positional
# shape      : any
# description: the new value to merge with


# This is the custom command 1 for merge:

#[test]
def merge_add_an_index_column_to_the_input_table_1 [] {
  let result = ([a b c] | wrap name | merge ( [1 2 3] | wrap index ))
  assert ($result == [{name: a, index: 1}, {name: b, index: 2}, {name: c, index: 3}])
}

# This is the custom command 2 for merge:

#[test]
def merge_merge_two_records_2 [] {
  let result = ({a: 1, b: 2} | merge {c: 3})
  assert ($result == {a: 1, b: 2, c: 3})
}

# This is the custom command 3 for merge:

#[test]
def merge_merge_two_tables_overwriting_overlapping_columns_3 [] {
  let result = ([{columnA: A0 columnB: B0}] | merge [{columnA: 'A0*'}])
  assert ($result == [{columnA: A0*, columnB: B0}])
}


