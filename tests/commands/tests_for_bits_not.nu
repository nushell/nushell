use std assert

# Parameter name:
# sig type   : int
# name       : signed
# type       : switch
# shape      : 
# description: always treat input number as a signed number

# Parameter name:
# sig type   : int
# name       : number-bytes
# type       : named
# shape      : string
# description: the size of unsigned number in bytes, it can be 1, 2, 4, 8, auto

# Parameter name:
# sig type   : list<int>
# name       : signed
# type       : switch
# shape      : 
# description: always treat input number as a signed number

# Parameter name:
# sig type   : list<int>
# name       : number-bytes
# type       : named
# shape      : string
# description: the size of unsigned number in bytes, it can be 1, 2, 4, 8, auto


# This is the custom command 1 for bits_not:

#[test]
def bits_not_apply_logical_negation_to_a_list_of_numbers_1 [] {
  let result = ([4 3 2] | bits not)
  assert ($result == [140737488355323, 140737488355324, 140737488355325])
}

# This is the custom command 2 for bits_not:

#[test]
def bits_not_apply_logical_negation_to_a_list_of_numbers_treat_input_as_2_bytes_number_2 [] {
  let result = ([4 3 2] | bits not -n '2')
  assert ($result == [65531, 65532, 65533])
}

# This is the custom command 3 for bits_not:

#[test]
def bits_not_apply_logical_negation_to_a_list_of_numbers_treat_input_as_signed_number_3 [] {
  let result = ([4 3 2] | bits not -s)
  assert ($result == [-5, -4, -3])
}


