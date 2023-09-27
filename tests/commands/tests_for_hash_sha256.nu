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


# This is the custom command 1 for hash_sha256:

#[test]
def hash_sha256_return_the_sha256_hash_of_a_string_hex_encoded_1 [] {
  let result = ('abcdefghijklmnopqrstuvwxyz' | hash sha256)
  assert ($result == 71c480df93d6ae2f1efad1447c66c9525e316218cf51fc8d9ed832f2daf18b73)
}

# This is the custom command 2 for hash_sha256:

#[test]
def hash_sha256_return_the_sha256_hash_of_a_string_as_binary_2 [] {
  let result = ('abcdefghijklmnopqrstuvwxyz' | hash sha256 --binary)
  assert ($result == [113, 196, 128, 223, 147, 214, 174, 47, 30, 250, 209, 68, 124, 102, 201, 82, 94, 49, 98, 24, 207, 81, 252, 141, 158, 216, 50, 242, 218, 241, 139, 115])
}

# This is the custom command 3 for hash_sha256:

#[test]
def hash_sha256_return_the_sha256_hash_of_a_files_contents_3 [] {
  let result = (open ./nu_0_24_1_windows.zip | hash sha256)
  assert ($result == )
}


