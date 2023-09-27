use std assert

# Parameter name:
# sig type   : record
# name       : binary
# type       : switch
# shape      : 
# description: Output binary instead of hexadecimal representation

# Parameter name:
# sig type   : string
# name       : binary
# type       : switch
# shape      : 
# description: Output binary instead of hexadecimal representation

# Parameter name:
# sig type   : table
# name       : binary
# type       : switch
# shape      : 
# description: Output binary instead of hexadecimal representation


# This is the custom command 1 for hash_md5:

#[test]
def hash_md5_return_the_md5_hash_of_a_string_hex_encoded_1 [] {
  let result = ('abcdefghijklmnopqrstuvwxyz' | hash md5)
  assert ($result == c3fcd3d76192e4007dfb496cca67e13b)
}

# This is the custom command 2 for hash_md5:

#[test]
def hash_md5_return_the_md5_hash_of_a_string_as_binary_2 [] {
  let result = ('abcdefghijklmnopqrstuvwxyz' | hash md5 --binary)
  assert ($result == [195, 252, 211, 215, 97, 146, 228, 0, 125, 251, 73, 108, 202, 103, 225, 59])
}

# This is the custom command 3 for hash_md5:

#[test]
def hash_md5_return_the_md5_hash_of_a_files_contents_3 [] {
  let result = (open ./nu_0_24_1_windows.zip | hash md5)
  assert ($result == )
}


