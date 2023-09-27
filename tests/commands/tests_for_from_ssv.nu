use std assert

# Parameter name:
# sig type   : string
# name       : noheaders
# type       : switch
# shape      : 
# description: don't treat the first row as column names

# Parameter name:
# sig type   : string
# name       : aligned-columns
# type       : switch
# shape      : 
# description: assume columns are aligned

# Parameter name:
# sig type   : string
# name       : minimum-spaces
# type       : named
# shape      : int
# description: the minimum spaces to separate columns


# This is the custom command 1 for from_ssv:

#[test]
def from_ssv_converts_ssv_formatted_string_to_table_1 [] {
  let result = ('FOO   BAR
1   2' | from ssv)
  assert ($result == [{FOO: 1, BAR: 2}])
}

# This is the custom command 2 for from_ssv:

#[test]
def from_ssv_converts_ssv_formatted_string_to_table_but_not_treating_the_first_row_as_column_names_2 [] {
  let result = ('FOO   BAR
1   2' | from ssv -n)
  assert ($result == [{column1: FOO, column2: BAR}, {column1: 1, column2: 2}])
}


