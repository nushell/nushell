use std assert

# Parameter name:
# sig type   : any
# name       : row
# type       : positional
# shape      : any
# description: the row, list, or table to append


# This is the custom command 1 for append:

#[test]
def append_append_one_integer_to_a_list_1 [] {
  let result = ([0 1 2 3] | append 4)
  assert ($result == [0, 1, 2, 3, 4])
}

# This is the custom command 2 for append:

#[test]
def append_append_a_list_to_an_item_2 [] {
  let result = (0 | append [1 2 3])
  assert ($result == [0, 1, 2, 3])
}

# This is the custom command 3 for append:

#[test]
def append_append_a_list_of_string_to_a_string_3 [] {
  let result = ("a" | append ["b"] )
  assert ($result == [a, b])
}

# This is the custom command 4 for append:

#[test]
def append_append_three_integer_items_4 [] {
  let result = ([0 1] | append [2 3 4])
  assert ($result == [0, 1, 2, 3, 4])
}

# This is the custom command 5 for append:

#[test]
def append_append_integers_and_strings_5 [] {
  let result = ([0 1] | append [2 nu 4 shell])
  assert ($result == [0, 1, 2, nu, 4, shell])
}

# This is the custom command 6 for append:

#[test]
def append_append_a_range_of_integers_to_a_list_6 [] {
  let result = ([0 1] | append 2..4)
  assert ($result == [0, 1, 2, 3, 4])
}


