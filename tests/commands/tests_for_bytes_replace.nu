use std assert

# Parameter name:
# sig type   : binary
# name       : find
# type       : positional
# shape      : binary
# description: the pattern to find

# Parameter name:
# sig type   : binary
# name       : replace
# type       : positional
# shape      : binary
# description: the replacement pattern

# Parameter name:
# sig type   : binary
# name       : all
# type       : switch
# shape      : 
# description: replace all occurrences of find binary

# Parameter name:
# sig type   : record
# name       : find
# type       : positional
# shape      : binary
# description: the pattern to find

# Parameter name:
# sig type   : record
# name       : replace
# type       : positional
# shape      : binary
# description: the replacement pattern

# Parameter name:
# sig type   : record
# name       : all
# type       : switch
# shape      : 
# description: replace all occurrences of find binary

# Parameter name:
# sig type   : table
# name       : find
# type       : positional
# shape      : binary
# description: the pattern to find

# Parameter name:
# sig type   : table
# name       : replace
# type       : positional
# shape      : binary
# description: the replacement pattern

# Parameter name:
# sig type   : table
# name       : all
# type       : switch
# shape      : 
# description: replace all occurrences of find binary


# This is the custom command 1 for bytes_replace:

#[test]
def bytes_replace_find_and_replace_contents_1 [] {
  let result = (0x[10 AA FF AA FF] | bytes replace 0x[10 AA] 0x[FF])
  assert ($result == [255, 255, 170, 255])
}

# This is the custom command 2 for bytes_replace:

#[test]
def bytes_replace_find_and_replace_all_occurrences_of_find_binary_2 [] {
  let result = (0x[10 AA 10 BB 10] | bytes replace -a 0x[10] 0x[A0])
  assert ($result == [160, 170, 160, 187, 160])
}

# This is the custom command 3 for bytes_replace:

#[test]
def bytes_replace_find_and_replace_all_occurrences_of_find_binary_in_table_3 [] {
  let result = ([[ColA ColB ColC]; [0x[11 12 13] 0x[14 15 16] 0x[17 18 19]]] | bytes replace -a 0x[11] 0x[13] ColA ColC)
  assert ($result == [{ColA: [19, 18, 19], ColB: [20, 21, 22], ColC: [23, 24, 25]}])
}


