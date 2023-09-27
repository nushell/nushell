use std assert


# This is the custom command 1 for bytes_length:

#[test]
def bytes_length_return_the_length_of_a_binary_1 [] {
  let result = (0x[1F FF AA AB] | bytes length)
  assert ($result == 4)
}

# This is the custom command 2 for bytes_length:

#[test]
def bytes_length_return_the_lengths_of_multiple_binaries_2 [] {
  let result = ([0x[1F FF AA AB] 0x[1F]] | bytes length)
  assert ($result == [4, 1])
}


