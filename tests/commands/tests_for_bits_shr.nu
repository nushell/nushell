use std assert

# Parameter name:
# sig type   : int
# name       : bits
# type       : positional
# shape      : int
# description: number of bits to shift right

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
# description: number of bits to shift right

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


# This is the custom command 1 for bits_shr:

#[test]
def bits_shr_shift_right_a_number_with_2_bits_1 [] {
  let result = (8 | bits shr 2)
  assert ($result == 2)
}

# This is the custom command 2 for bits_shr:

#[test]
def bits_shr_shift_right_a_list_of_numbers_2 [] {
  let result = ([15 35 2] | bits shr 2)
  assert ($result == [3, 8, 0])
}


