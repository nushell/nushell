use std assert

# Parameter name:
# sig type   : binary
# name       : range
# type       : positional
# shape      : range
# description: the range to get bytes

# Parameter name:
# sig type   : list<binary>
# name       : range
# type       : positional
# shape      : range
# description: the range to get bytes

# Parameter name:
# sig type   : record
# name       : range
# type       : positional
# shape      : range
# description: the range to get bytes

# Parameter name:
# sig type   : table
# name       : range
# type       : positional
# shape      : range
# description: the range to get bytes


# This is the custom command 1 for bytes_at:

#[test]
def bytes_at_get_a_subbytes_0x10_01_from_the_bytes_0x33_44_55_10_01_13_1 [] {
  let result = ( 0x[33 44 55 10 01 13] | bytes at 3..<4)
  assert ($result == [16])
}

# This is the custom command 2 for bytes_at:

#[test]
def bytes_at_get_a_subbytes_0x10_01_13_from_the_bytes_0x33_44_55_10_01_13_2 [] {
  let result = ( 0x[33 44 55 10 01 13] | bytes at 3..6)
  assert ($result == [16, 1, 19])
}

# This is the custom command 3 for bytes_at:

#[test]
def bytes_at_get_the_remaining_characters_from_a_starting_index_3 [] {
  let result = ( { data: 0x[33 44 55 10 01 13] } | bytes at 3.. data)
  assert ($result == {data: [16, 1, 19]})
}

# This is the custom command 4 for bytes_at:

#[test]
def bytes_at_get_the_characters_from_the_beginning_until_ending_index_4 [] {
  let result = ( 0x[33 44 55 10 01 13] | bytes at ..<4)
  assert ($result == [51, 68, 85, 16])
}

# This is the custom command 5 for bytes_at:

#[test]
def bytes_at_or_the_characters_from_the_beginning_until_ending_index_inside_a_table_5 [] {
  let result = ( [[ColA ColB ColC]; [0x[11 12 13] 0x[14 15 16] 0x[17 18 19]]] | bytes at 1.. ColB ColC)
  assert ($result == [{ColA: [17, 18, 19], ColB: [21, 22], ColC: [24, 25]}])
}


