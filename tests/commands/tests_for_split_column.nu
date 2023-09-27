use std assert

# Parameter name:
# sig type   : list<string>
# name       : separator
# type       : positional
# shape      : string
# description: the character or string that denotes what separates columns

# Parameter name:
# sig type   : list<string>
# name       : collapse-empty
# type       : switch
# shape      : 
# description: remove empty columns

# Parameter name:
# sig type   : list<string>
# name       : regex
# type       : switch
# shape      : 
# description: separator is a regular expression

# Parameter name:
# sig type   : string
# name       : separator
# type       : positional
# shape      : string
# description: the character or string that denotes what separates columns

# Parameter name:
# sig type   : string
# name       : collapse-empty
# type       : switch
# shape      : 
# description: remove empty columns

# Parameter name:
# sig type   : string
# name       : regex
# type       : switch
# shape      : 
# description: separator is a regular expression


# This is the custom command 1 for split_column:

#[test]
def split_column_split_a_string_into_columns_by_the_specified_separator_1 [] {
  let result = ('a--b--c' | split column '--')
  assert ($result == [{column1: a, column2: b, column3: c}])
}

# This is the custom command 2 for split_column:

#[test]
def split_column_split_a_string_into_columns_of_char_and_remove_the_empty_columns_2 [] {
  let result = ('abc' | split column -c '')
  assert ($result == [{column1: a, column2: b, column3: c}])
}

# This is the custom command 3 for split_column:

#[test]
def split_column_split_a_list_of_strings_into_a_table_3 [] {
  let result = (['a-b' 'c-d'] | split column -)
  assert ($result == [{column1: a, column2: b}, {column1: c, column2: d}])
}

# This is the custom command 4 for split_column:

#[test]
def split_column_split_a_list_of_strings_into_a_table_ignoring_padding_4 [] {
  let result = (['a -  b' 'c  -    d'] | split column -r '\s*-\s*')
  assert ($result == [{column1: a, column2: b}, {column1: c, column2: d}])
}


