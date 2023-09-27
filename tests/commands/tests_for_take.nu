use std assert

# Parameter name:
# sig type   : binary
# name       : n
# type       : positional
# shape      : int
# description: starting from the front, the number of elements to return

# Parameter name:
# sig type   : list<any>
# name       : n
# type       : positional
# shape      : int
# description: starting from the front, the number of elements to return

# Parameter name:
# sig type   : range
# name       : n
# type       : positional
# shape      : int
# description: starting from the front, the number of elements to return

# Parameter name:
# sig type   : table
# name       : n
# type       : positional
# shape      : int
# description: starting from the front, the number of elements to return


# This is the custom command 1 for take:

#[test]
def take_return_the_first_item_of_a_listtable_1 [] {
  let result = ([1 2 3] | take 1)
  assert ($result == [1])
}

# This is the custom command 2 for take:

#[test]
def take_return_the_first_2_items_of_a_listtable_2 [] {
  let result = ([1 2 3] | take 2)
  assert ($result == [1, 2])
}

# This is the custom command 3 for take:

#[test]
def take_return_the_first_two_rows_of_a_table_3 [] {
  let result = ([[editions]; [2015] [2018] [2021]] | take 2)
  assert ($result == [{editions: 2015}, {editions: 2018}])
}

# This is the custom command 4 for take:

#[test]
def take_return_the_first_2_bytes_of_a_binary_value_4 [] {
  let result = (0x[01 23 45] | take 2)
  assert ($result == [1, 35])
}

# This is the custom command 5 for take:

#[test]
def take_return_the_first_3_elements_of_a_range_5 [] {
  let result = (1..10 | take 3)
  assert ($result == [1, 2, 3])
}


