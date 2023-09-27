use std assert

# Parameter name:
# sig type   : table
# name       : columns
# type       : positional
# shape      : int
# description: starting from the end, the number of columns to remove


# This is the custom command 1 for drop_column:

#[test]
def drop_column_remove_the_last_column_of_a_table_1 [] {
  let result = ([[lib, extension]; [nu-lib, rs] [nu-core, rb]] | drop column)
  assert ($result == [{lib: nu-lib}, {lib: nu-core}])
}


