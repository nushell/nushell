use std assert


# This is the custom command 1 for encode_hex:

#[test]
def encode_hex_encode_binary_data_1 [] {
  let result = (0x[09 F9 11 02 9D 74 E3 5B D8 41 56 C5 63 56 88 C0] | encode hex)
  assert ($result == 09F911029D74E35BD84156C5635688C0)
}


