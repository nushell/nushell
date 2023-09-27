use std assert

# Parameter name:
# sig type   : string
# name       : sheets
# type       : named
# shape      : list<string>
# description: Only convert specified sheets


# This is the custom command 1 for from_ods:

#[test]
def from_ods_convert_binary_ods_data_to_a_table_1 [] {
  let result = (open --raw test.ods | from ods)
  assert ($result == )
}

# This is the custom command 2 for from_ods:

#[test]
def from_ods_convert_binary_ods_data_to_a_table_specifying_the_tables_2 [] {
  let result = (open --raw test.ods | from ods -s [Spreadsheet1])
  assert ($result == )
}


