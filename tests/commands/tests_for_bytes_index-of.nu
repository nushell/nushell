use std assert

# Parameter name:
# sig type   : binary
# name       : pattern
# type       : positional
# shape      : binary
# description: the pattern to find index of

# Parameter name:
# sig type   : binary
# name       : all
# type       : switch
# shape      : 
# description: returns all matched index

# Parameter name:
# sig type   : binary
# name       : end
# type       : switch
# shape      : 
# description: search from the end of the binary

# Parameter name:
# sig type   : record
# name       : pattern
# type       : positional
# shape      : binary
# description: the pattern to find index of

# Parameter name:
# sig type   : record
# name       : all
# type       : switch
# shape      : 
# description: returns all matched index

# Parameter name:
# sig type   : record
# name       : end
# type       : switch
# shape      : 
# description: search from the end of the binary

# Parameter name:
# sig type   : table
# name       : pattern
# type       : positional
# shape      : binary
# description: the pattern to find index of

# Parameter name:
# sig type   : table
# name       : all
# type       : switch
# shape      : 
# description: returns all matched index

# Parameter name:
# sig type   : table
# name       : end
# type       : switch
# shape      : 
# description: search from the end of the binary


# This is the custom command 1 for bytes_index-of:

#[test]
def bytes_index-of_returns_index_of_pattern_in_bytes_1 [] {
  let result = ( 0x[33 44 55 10 01 13 44 55] | bytes index-of 0x[44 55])
  assert ($result == 1)
}

# This is the custom command 2 for bytes_index-of:

#[test]
def bytes_index-of_returns_index_of_pattern_search_from_end_2 [] {
  let result = ( 0x[33 44 55 10 01 13 44 55] | bytes index-of -e 0x[44 55])
  assert ($result == 6)
}

# This is the custom command 3 for bytes_index-of:

#[test]
def bytes_index-of_returns_all_matched_index_3 [] {
  let result = ( 0x[33 44 55 10 01 33 44 33 44] | bytes index-of -a 0x[33 44])
  assert ($result == [0, 5, 7])
}

# This is the custom command 4 for bytes_index-of:

#[test]
def bytes_index-of_returns_all_matched_index_searching_from_end_4 [] {
  let result = ( 0x[33 44 55 10 01 33 44 33 44] | bytes index-of -a -e 0x[33 44])
  assert ($result == [7, 5, 0])
}

# This is the custom command 5 for bytes_index-of:

#[test]
def bytes_index-of_returns_index_of_pattern_for_specific_column_5 [] {
  let result = ( [[ColA ColB ColC]; [0x[11 12 13] 0x[14 15 16] 0x[17 18 19]]] | bytes index-of 0x[11] ColA ColC)
  assert ($result == [{ColA: 0, ColB: [20, 21, 22], ColC: -1}])
}


