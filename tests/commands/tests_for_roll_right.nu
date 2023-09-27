use std assert

# Parameter name:
# sig type   : record
# name       : by
# type       : named
# shape      : int
# description: Number of columns to roll

# Parameter name:
# sig type   : record
# name       : cells-only
# type       : switch
# shape      : 
# description: rotates columns leaving headers fixed

# Parameter name:
# sig type   : table
# name       : by
# type       : named
# shape      : int
# description: Number of columns to roll

# Parameter name:
# sig type   : table
# name       : cells-only
# type       : switch
# shape      : 
# description: rotates columns leaving headers fixed


# This is the custom command 1 for roll_right:

#[test]
def roll_right_rolls_columns_of_a_record_to_the_right_1 [] {
  let result = ({a:1 b:2 c:3} | roll right)
  assert ($result == {c: 3, a: 1, b: 2})
}

# This is the custom command 2 for roll_right:

#[test]
def roll_right_rolls_columns_to_the_right_2 [] {
  let result = ([[a b c]; [1 2 3] [4 5 6]] | roll right)
  assert ($result == [{c: 3, a: 1, b: 2}, {c: 6, a: 4, b: 5}])
}

# This is the custom command 3 for roll_right:

#[test]
def roll_right_rolls_columns_to_the_right_with_fixed_headers_3 [] {
  let result = ([[a b c]; [1 2 3] [4 5 6]] | roll right --cells-only)
  assert ($result == [{a: 3, b: 1, c: 2}, {a: 6, b: 4, c: 5}])
}


