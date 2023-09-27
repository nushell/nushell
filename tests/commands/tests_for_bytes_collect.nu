use std assert

# Parameter name:
# sig type   : list<binary>
# name       : separator
# type       : positional
# shape      : binary
# description: optional separator to use when creating binary


# This is the custom command 1 for bytes_collect:

#[test]
def bytes_collect_create_a_byte_array_from_input_1 [] {
  let result = ([0x[11] 0x[13 15]] | bytes collect)
  assert ($result == [17, 19, 21])
}

# This is the custom command 2 for bytes_collect:

#[test]
def bytes_collect_create_a_byte_array_from_input_with_a_separator_2 [] {
  let result = ([0x[11] 0x[33] 0x[44]] | bytes collect 0x[01])
  assert ($result == [17, 1, 51, 1, 68])
}


