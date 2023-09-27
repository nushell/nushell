use std assert

# Parameter name:
# sig type   : table
# name       : by
# type       : named
# shape      : int
# description: Number of rows to roll


# This is the custom command 1 for roll_up:

#[test]
def roll_up_rolls_rows_up_1 [] {
  let result = ([[a b]; [1 2] [3 4] [5 6]] | roll up)
  assert ($result == [{a: 3, b: 4}, {a: 5, b: 6}, {a: 1, b: 2}])
}


