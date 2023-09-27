use std assert

# Parameter name:
# sig type   : binary
# name       : encoding
# type       : positional
# shape      : string
# description: the text encoding to use


# This is the custom command 1 for decode:

#[test]
def decode_decode_the_output_of_an_external_command_1 [] {
  let result = (^cat myfile.q | decode utf-8)
  assert ($result == )
}

# This is the custom command 2 for decode:

#[test]
def decode_decode_an_utf_16_string_into_nushell_utf_8_string_2 [] {
  let result = (0x[00 53 00 6F 00 6D 00 65 00 20 00 44 00 61 00 74 00 61] | decode utf-16be)
  assert ($result == Some Data)
}


