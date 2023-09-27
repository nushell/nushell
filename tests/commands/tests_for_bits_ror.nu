use std assert

# Parameter name:
# sig type   : int
# name       : bits
# type       : positional
# shape      : int
# description: number of bits to rotate right

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
# description: number of bits to rotate right

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


# This is the custom command 1 for bits_ror:

#[test]
def bits_ror_rotate_right_a_number_with_60_bits_1 [] {
  let result = (17 | bits ror 60)
  assert ($result == 272)
}

# This is the custom command 2 for bits_ror:

#[test]
def bits_ror_rotate_right_a_list_of_numbers_of_one_byte_2 [] {
  let result = ([15 33 92] | bits ror 2 -n '1')
  assert ($result == [195, 72, 23])
}


