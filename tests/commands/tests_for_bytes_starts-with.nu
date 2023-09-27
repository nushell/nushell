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


# This is the custom command 1 for bytes_starts-with:

#[test]
def bytes_starts-with_checks_if_binary_starts_with_0x1f_ff_aa_1 [] {
  let result = (0x[1F FF AA AA] | bytes starts-with 0x[1F FF AA])
  assert ($result == true)
}

# This is the custom command 2 for bytes_starts-with:

#[test]
def bytes_starts-with_checks_if_binary_starts_with_0x1f_2 [] {
  let result = (0x[1F FF AA AA] | bytes starts-with 0x[1F])
  assert ($result == true)
}

# This is the custom command 3 for bytes_starts-with:

#[test]
def bytes_starts-with_checks_if_binary_starts_with_0x1f_3 [] {
  let result = (0x[1F FF AA AA] | bytes starts-with 0x[11])
  assert ($result == false)
}


