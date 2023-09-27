use std assert

# Parameter name:
# sig type   : int
# name       : target
# type       : positional
# shape      : int
# description: target integer to perform bit or

# Parameter name:
# sig type   : list<int>
# name       : target
# type       : positional
# shape      : int
# description: target integer to perform bit or


# This is the custom command 1 for bits_or:

#[test]
def bits_or_apply_bits_or_to_two_numbers_1 [] {
  let result = (2 | bits or 6)
  assert ($result == 6)
}

# This is the custom command 2 for bits_or:

#[test]
def bits_or_apply_logical_or_to_a_list_of_numbers_2 [] {
  let result = ([8 3 2] | bits or 2)
  assert ($result == [10, 3, 2])
}


