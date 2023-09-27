use std assert

# Parameter name:
# sig type   : binary
# name       : sheets
# type       : named
# shape      : list<string>
# description: Only convert specified sheets


# This is the custom command 1 for from_xlsx:

#[test]
def from_xlsx_convert_binary_xlsx_data_to_a_table_1 [] {
  let result = (open --raw test.xlsx | from xlsx)
  assert ($result == )
}

# This is the custom command 2 for from_xlsx:

#[test]
def from_xlsx_convert_binary_xlsx_data_to_a_table_specifying_the_tables_2 [] {
  let result = (open --raw test.xlsx | from xlsx -s [Spreadsheet1])
  assert ($result == )
}


