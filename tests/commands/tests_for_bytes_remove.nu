use std assert

# Parameter name:
# sig type   : binary
# name       : pattern
# type       : positional
# shape      : binary
# description: the pattern to find

# Parameter name:
# sig type   : binary
# name       : end
# type       : switch
# shape      : 
# description: remove from end of binary

# Parameter name:
# sig type   : binary
# name       : all
# type       : switch
# shape      : 
# description: remove occurrences of finding binary

# Parameter name:
# sig type   : record
# name       : pattern
# type       : positional
# shape      : binary
# description: the pattern to find

# Parameter name:
# sig type   : record
# name       : end
# type       : switch
# shape      : 
# description: remove from end of binary

# Parameter name:
# sig type   : record
# name       : all
# type       : switch
# shape      : 
# description: remove occurrences of finding binary

# Parameter name:
# sig type   : table
# name       : pattern
# type       : positional
# shape      : binary
# description: the pattern to find

# Parameter name:
# sig type   : table
# name       : end
# type       : switch
# shape      : 
# description: remove from end of binary

# Parameter name:
# sig type   : table
# name       : all
# type       : switch
# shape      : 
# description: remove occurrences of finding binary


# This is the custom command 1 for bytes_remove:

#[test]
def bytes_remove_remove_contents_1 [] {
  let result = (0x[10 AA FF AA FF] | bytes remove 0x[10 AA])
  assert ($result == [255, 170, 255])
}

# This is the custom command 2 for bytes_remove:

#[test]
def bytes_remove_remove_all_occurrences_of_find_binary_in_record_field_2 [] {
  let result = ({ data: 0x[10 AA 10 BB 10] } | bytes remove -a 0x[10] data)
  assert ($result == {data: [170, 187]})
}

# This is the custom command 3 for bytes_remove:

#[test]
def bytes_remove_remove_occurrences_of_find_binary_from_end_3 [] {
  let result = (0x[10 AA 10 BB CC AA 10] | bytes remove -e 0x[10])
  assert ($result == [16, 170, 16, 187, 204, 170])
}

# This is the custom command 4 for bytes_remove:

#[test]
def bytes_remove_remove_all_occurrences_of_find_binary_in_table_4 [] {
  let result = ([[ColA ColB ColC]; [0x[11 12 13] 0x[14 15 16] 0x[17 18 19]]] | bytes remove 0x[11] ColA ColC)
  assert ($result == [{ColA: [18, 19], ColB: [20, 21, 22], ColC: [23, 24, 25]}])
}


