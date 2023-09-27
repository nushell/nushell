use std assert

# Parameter name:
# sig type   : list<any>
# name       : rows
# type       : positional
# shape      : int
# description: The number of items to remove

# Parameter name:
# sig type   : table
# name       : rows
# type       : positional
# shape      : int
# description: The number of items to remove


# This is the custom command 1 for drop:

#[test]
def drop_remove_the_last_item_of_a_list_1 [] {
  let result = ([0,1,2,3] | drop)
  assert ($result == [0, 1, 2])
}

# This is the custom command 2 for drop:

#[test]
def drop_remove_zero_item_of_a_list_2 [] {
  let result = ([0,1,2,3] | drop 0)
  assert ($result == [0, 1, 2, 3])
}

# This is the custom command 3 for drop:

#[test]
def drop_remove_the_last_two_items_of_a_list_3 [] {
  let result = ([0,1,2,3] | drop 2)
  assert ($result == [0, 1])
}

# This is the custom command 4 for drop:

#[test]
def drop_remove_the_last_row_in_a_table_4 [] {
  let result = ([[a, b]; [1, 2] [3, 4]] | drop 1)
  assert ($result == [{a: 1, b: 2}])
}


