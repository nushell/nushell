use std assert

# Parameter name:
# sig type   : table
# name       : columns
# type       : named
# shape      : table
# description: list of columns to update


# This is the custom command 1 for into_value:

#[test]
def into_value_infer_nushell_values_for_each_cell_1 [] {
  let result = ($table | into value)
  assert ($result == )
}

# This is the custom command 2 for into_value:

#[test]
def into_value_infer_nushell_values_for_each_cell_in_the_given_columns_2 [] {
  let result = ($table | into value -c [column1, column5])
  assert ($result == )
}


