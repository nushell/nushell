use std assert

# Parameter name:
# sig type   : int
# name       : target
# type       : positional
# shape      : int
# description: target integer to perform bit xor

# Parameter name:
# sig type   : list<int>
# name       : target
# type       : positional
# shape      : int
# description: target integer to perform bit xor


# This is the custom command 1 for bits_xor:

#[test]
def bits_xor_apply_bits_xor_to_two_numbers_1 [] {
  let result = (2 | bits xor 2)
  assert ($result == 0)
}

# This is the custom command 2 for bits_xor:

#[test]
def bits_xor_apply_logical_xor_to_a_list_of_numbers_2 [] {
  let result = ([8 3 2] | bits xor 2)
  assert ($result == [10, 1, 0])
}


