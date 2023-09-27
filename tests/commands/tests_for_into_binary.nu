use std assert

# Parameter name:
# sig type   : binary
# name       : compact
# type       : switch
# shape      : 
# description: output without padding zeros

# Parameter name:
# sig type   : bool
# name       : compact
# type       : switch
# shape      : 
# description: output without padding zeros

# Parameter name:
# sig type   : datetime
# name       : compact
# type       : switch
# shape      : 
# description: output without padding zeros

# Parameter name:
# sig type   : filesize
# name       : compact
# type       : switch
# shape      : 
# description: output without padding zeros

# Parameter name:
# sig type   : int
# name       : compact
# type       : switch
# shape      : 
# description: output without padding zeros

# Parameter name:
# sig type   : number
# name       : compact
# type       : switch
# shape      : 
# description: output without padding zeros

# Parameter name:
# sig type   : record
# name       : compact
# type       : switch
# shape      : 
# description: output without padding zeros

# Parameter name:
# sig type   : string
# name       : compact
# type       : switch
# shape      : 
# description: output without padding zeros

# Parameter name:
# sig type   : table
# name       : compact
# type       : switch
# shape      : 
# description: output without padding zeros


# This is the custom command 1 for into_binary:

#[test]
def into_binary_convert_string_to_a_nushell_binary_primitive_1 [] {
  let result = ('This is a string that is exactly 52 characters long.' | into binary)
  assert ($result == [84, 104, 105, 115, 32, 105, 115, 32, 97, 32, 115, 116, 114, 105, 110, 103, 32, 116, 104, 97, 116, 32, 105, 115, 32, 101, 120, 97, 99, 116, 108, 121, 32, 53, 50, 32, 99, 104, 97, 114, 97, 99, 116, 101, 114, 115, 32, 108, 111, 110, 103, 46])
}

# This is the custom command 2 for into_binary:

#[test]
def into_binary_convert_a_number_to_a_nushell_binary_primitive_2 [] {
  let result = (1 | into binary)
  assert ($result == [1, 0, 0, 0, 0, 0, 0, 0])
}

# This is the custom command 3 for into_binary:

#[test]
def into_binary_convert_a_boolean_to_a_nushell_binary_primitive_3 [] {
  let result = (true | into binary)
  assert ($result == [1, 0, 0, 0, 0, 0, 0, 0])
}

# This is the custom command 4 for into_binary:

#[test]
def into_binary_convert_a_filesize_to_a_nushell_binary_primitive_4 [] {
  let result = (ls | where name == LICENSE | get size | into binary)
  assert ($result == )
}

# This is the custom command 5 for into_binary:

#[test]
def into_binary_convert_a_filepath_to_a_nushell_binary_primitive_5 [] {
  let result = (ls | where name == LICENSE | get name | path expand | into binary)
  assert ($result == )
}

# This is the custom command 6 for into_binary:

#[test]
def into_binary_convert_a_float_to_a_nushell_binary_primitive_6 [] {
  let result = (1.234 | into binary)
  assert ($result == [88, 57, 180, 200, 118, 190, 243, 63])
}

# This is the custom command 7 for into_binary:

#[test]
def into_binary_convert_an_integer_to_a_nushell_binary_primitive_with_compact_enabled_7 [] {
  let result = (10 | into binary --compact)
  assert ($result == [10])
}


