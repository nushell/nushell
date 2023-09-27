use std assert

# Parameter name:
# sig type   : any
# name       : row
# type       : positional
# shape      : any
# description: the row, list, or table to prepend


# This is the custom command 1 for prepend:

#[test]
def prepend_prepend_a_list_to_an_item_1 [] {
  let result = (0 | prepend [1 2 3])
  assert ($result == [1, 2, 3, 0])
}

# This is the custom command 2 for prepend:

#[test]
def prepend_prepend_a_list_of_strings_to_a_string_2 [] {
  let result = ("a" | prepend ["b"] )
  assert ($result == [b, a])
}

# This is the custom command 3 for prepend:

#[test]
def prepend_prepend_one_integer_item_3 [] {
  let result = ([1 2 3 4] | prepend 0)
  assert ($result == [0, 1, 2, 3, 4])
}

# This is the custom command 4 for prepend:

#[test]
def prepend_prepend_two_integer_items_4 [] {
  let result = ([2 3 4] | prepend [0 1])
  assert ($result == [0, 1, 2, 3, 4])
}

# This is the custom command 5 for prepend:

#[test]
def prepend_prepend_integers_and_strings_5 [] {
  let result = ([2 nu 4 shell] | prepend [0 1 rocks])
  assert ($result == [0, 1, rocks, 2, nu, 4, shell])
}

# This is the custom command 6 for prepend:

#[test]
def prepend_prepend_a_range_6 [] {
  let result = ([3 4] | prepend 0..2)
  assert ($result == [0, 1, 2, 3, 4])
}


