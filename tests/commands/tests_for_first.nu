use std assert

# Parameter name:
# sig type   : binary
# name       : rows
# type       : positional
# shape      : int
# description: starting from the front, the number of rows to return

# Parameter name:
# sig type   : list<any>
# name       : rows
# type       : positional
# shape      : int
# description: starting from the front, the number of rows to return

# Parameter name:
# sig type   : range
# name       : rows
# type       : positional
# shape      : int
# description: starting from the front, the number of rows to return


# This is the custom command 1 for first:

#[test]
def first_return_the_first_item_of_a_listtable_1 [] {
  let result = ([1 2 3] | first)
  assert ($result == 1)
}

# This is the custom command 2 for first:

#[test]
def first_return_the_first_2_items_of_a_listtable_2 [] {
  let result = ([1 2 3] | first 2)
  assert ($result == [1, 2])
}

# This is the custom command 3 for first:

#[test]
def first_return_the_first_2_bytes_of_a_binary_value_3 [] {
  let result = (0x[01 23 45] | first 2)
  assert ($result == [1, 35])
}


