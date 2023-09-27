use std assert


# This is the custom command 1 for decode_hex:

#[test]
def decode_hex_hex_decode_a_value_and_output_as_binary_1 [] {
  let result = ('0102030A0a0B' | decode hex)
  assert ($result == [1, 2, 3, 10, 10, 11])
}

# This is the custom command 2 for decode_hex:

#[test]
def decode_hex_whitespaces_are_allowed_to_be_between_hex_digits_2 [] {
  let result = ('01 02  03 0A 0a 0B' | decode hex)
  assert ($result == [1, 2, 3, 10, 10, 11])
}


