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


# This is the custom command 1 for roll_left:

#[test]
def roll_left_rolls_columns_of_a_record_to_the_left_1 [] {
  let result = ({a:1 b:2 c:3} | roll left)
  assert ($result == {b: 2, c: 3, a: 1})
}

# This is the custom command 2 for roll_left:

#[test]
def roll_left_rolls_columns_of_a_table_to_the_left_2 [] {
  let result = ([[a b c]; [1 2 3] [4 5 6]] | roll left)
  assert ($result == [{b: 2, c: 3, a: 1}, {b: 5, c: 6, a: 4}])
}

# This is the custom command 3 for roll_left:

#[test]
def roll_left_rolls_columns_to_the_left_without_changing_column_names_3 [] {
  let result = ([[a b c]; [1 2 3] [4 5 6]] | roll left --cells-only)
  assert ($result == [{a: 2, b: 3, c: 1}, {a: 5, b: 6, c: 4}])
}


