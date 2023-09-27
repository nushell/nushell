use std assert

# Parameter name:
# sig type   : int
# name       : bits
# type       : positional
# shape      : int
# description: number of bits to rotate left

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
# description: the word size in number of bytes, it can be 1, 2, 4, 8, auto, default value `8`

# Parameter name:
# sig type   : list<int>
# name       : bits
# type       : positional
# shape      : int
# description: number of bits to rotate left

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
# description: the word size in number of bytes, it can be 1, 2, 4, 8, auto, default value `8`


# This is the custom command 1 for bits_rol:

#[test]
def bits_rol_rotate_left_a_number_with_2_bits_1 [] {
  let result = (17 | bits rol 2)
  assert ($result == 68)
}

# This is the custom command 2 for bits_rol:

#[test]
def bits_rol_rotate_left_a_list_of_numbers_with_2_bits_2 [] {
  let result = ([5 3 2] | bits rol 2)
  assert ($result == [20, 12, 8])
}


