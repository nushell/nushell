use std assert

# Parameter name:
# sig type   : int
# name       : target
# type       : positional
# shape      : int
# description: target integer to perform bit and

# Parameter name:
# sig type   : list<int>
# name       : target
# type       : positional
# shape      : int
# description: target integer to perform bit and


# This is the custom command 1 for bits_and:

#[test]
def bits_and_apply_bits_and_to_two_numbers_1 [] {
  let result = (2 | bits and 2)
  assert ($result == 2)
}

# This is the custom command 2 for bits_and:

#[test]
def bits_and_apply_logical_and_to_a_list_of_numbers_2 [] {
  let result = ([4 3 2] | bits and 2)
  assert ($result == [0, 2, 2])
}


