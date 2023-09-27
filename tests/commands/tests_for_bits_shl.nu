use std assert

# Parameter name:
# sig type   : int
# name       : bits
# type       : positional
# shape      : int
# description: number of bits to shift left

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
# description: number of bits to shift left

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


# This is the custom command 1 for bits_shl:

#[test]
def bits_shl_shift_left_a_number_by_7_bits_1 [] {
  let result = (2 | bits shl 7)
  assert ($result == 256)
}

# This is the custom command 2 for bits_shl:

#[test]
def bits_shl_shift_left_a_number_with_1_byte_by_7_bits_2 [] {
  let result = (2 | bits shl 7 -n '1')
  assert ($result == 0)
}

# This is the custom command 3 for bits_shl:

#[test]
def bits_shl_shift_left_a_signed_number_by_1_bit_3 [] {
  let result = (0x7F | bits shl 1 -s)
  assert ($result == 254)
}

# This is the custom command 4 for bits_shl:

#[test]
def bits_shl_shift_left_a_list_of_numbers_4 [] {
  let result = ([5 3 2] | bits shl 2)
  assert ($result == [20, 12, 8])
}


