use std assert

# Parameter name:
# sig type   : table
# name       : by
# type       : named
# shape      : int
# description: Number of rows to roll


# This is the custom command 1 for roll_down:

#[test]
def roll_down_rolls_rows_down_of_a_table_1 [] {
  let result = ([[a b]; [1 2] [3 4] [5 6]] | roll down)
  assert ($result == [{a: 5, b: 6}, {a: 1, b: 2}, {a: 3, b: 4}])
}


