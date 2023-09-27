use std assert

# Parameter name:
# sig type   : binary
# name       : rows
# type       : positional
# shape      : int
# description: starting from the back, the number of rows to return

# Parameter name:
# sig type   : list<any>
# name       : rows
# type       : positional
# shape      : int
# description: starting from the back, the number of rows to return


# This is the custom command 1 for last:

#[test]
def last_return_the_last_2_items_of_a_listtable_1 [] {
  let result = ([1,2,3] | last 2)
  assert ($result == [2, 3])
}

# This is the custom command 2 for last:

#[test]
def last_return_the_last_item_of_a_listtable_2 [] {
  let result = ([1,2,3] | last)
  assert ($result == 3)
}

# This is the custom command 3 for last:

#[test]
def last_return_the_last_2_bytes_of_a_binary_value_3 [] {
  let result = (0x[01 23 45] | last 2)
  assert ($result == [35, 69])
}


