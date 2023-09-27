use std assert

# Parameter name:
# sig type   : record
# name       : ccw
# type       : switch
# shape      : 
# description: rotate counter clockwise

# Parameter name:
# sig type   : table
# name       : ccw
# type       : switch
# shape      : 
# description: rotate counter clockwise


# This is the custom command 1 for rotate:

#[test]
def rotate_rotate_a_record_clockwise_producing_a_table_like_transpose_but_with_column_order_reversed_1 [] {
  let result = ({a:1, b:2} | rotate)
  assert ($result == [{column0: 1, column1: a}, {column0: 2, column1: b}])
}

# This is the custom command 2 for rotate:

#[test]
def rotate_rotate_2x3_table_clockwise_2 [] {
  let result = ([[a b]; [1 2] [3 4] [5 6]] | rotate)
  assert ($result == [{column0: 5, column1: 3, column2: 1, column3: a}, {column0: 6, column1: 4, column2: 2, column3: b}])
}

# This is the custom command 3 for rotate:

#[test]
def rotate_rotate_table_clockwise_and_change_columns_names_3 [] {
  let result = ([[a b]; [1 2]] | rotate col_a col_b)
  assert ($result == [{col_a: 1, col_b: a}, {col_a: 2, col_b: b}])
}

# This is the custom command 4 for rotate:

#[test]
def rotate_rotate_table_counter_clockwise_4 [] {
  let result = ([[a b]; [1 2]] | rotate --ccw)
  assert ($result == [{column0: b, column1: 2}, {column0: a, column1: 1}])
}

# This is the custom command 5 for rotate:

#[test]
def rotate_rotate_table_counter_clockwise_5 [] {
  let result = ([[a b]; [1 2] [3 4] [5 6]] | rotate --ccw)
  assert ($result == [{column0: b, column1: 2, column2: 4, column3: 6}, {column0: a, column1: 1, column2: 3, column3: 5}])
}

# This is the custom command 6 for rotate:

#[test]
def rotate_rotate_table_counter_clockwise_and_change_columns_names_6 [] {
  let result = ([[a b]; [1 2]] | rotate --ccw col_a col_b)
  assert ($result == [{col_a: b, col_b: 2}, {col_a: a, col_b: 1}])
}


