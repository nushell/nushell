use std assert

# Parameter name:
# sig type   : binary
# name       : pattern
# type       : positional
# shape      : binary
# description: the pattern to match

# Parameter name:
# sig type   : record
# name       : pattern
# type       : positional
# shape      : binary
# description: the pattern to match

# Parameter name:
# sig type   : table
# name       : pattern
# type       : positional
# shape      : binary
# description: the pattern to match


# This is the custom command 1 for bytes_ends-with:

#[test]
def bytes_ends-with_checks_if_binary_ends_with_0xaa_1 [] {
  let result = (0x[1F FF AA AA] | bytes ends-with 0x[AA])
  assert ($result == true)
}

# This is the custom command 2 for bytes_ends-with:

#[test]
def bytes_ends-with_checks_if_binary_ends_with_0xff_aa_aa_2 [] {
  let result = (0x[1F FF AA AA] | bytes ends-with 0x[FF AA AA])
  assert ($result == true)
}

# This is the custom command 3 for bytes_ends-with:

#[test]
def bytes_ends-with_checks_if_binary_ends_with_0x11_3 [] {
  let result = (0x[1F FF AA AA] | bytes ends-with 0x[11])
  assert ($result == false)
}


