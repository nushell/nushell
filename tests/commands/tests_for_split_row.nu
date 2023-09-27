use std assert

# Parameter name:
# sig type   : list<string>
# name       : separator
# type       : positional
# shape      : string
# description: a character or regex that denotes what separates rows

# Parameter name:
# sig type   : list<string>
# name       : number
# type       : named
# shape      : int
# description: Split into maximum number of items

# Parameter name:
# sig type   : list<string>
# name       : regex
# type       : switch
# shape      : 
# description: use regex syntax for separator

# Parameter name:
# sig type   : string
# name       : separator
# type       : positional
# shape      : string
# description: a character or regex that denotes what separates rows

# Parameter name:
# sig type   : string
# name       : number
# type       : named
# shape      : int
# description: Split into maximum number of items

# Parameter name:
# sig type   : string
# name       : regex
# type       : switch
# shape      : 
# description: use regex syntax for separator


# This is the custom command 1 for split_row:

#[test]
def split_row_split_a_string_into_rows_of_char_1 [] {
  let result = ('abc' | split row '')
  assert ($result == [, a, b, c, ])
}

# This is the custom command 2 for split_row:

#[test]
def split_row_split_a_string_into_rows_by_the_specified_separator_2 [] {
  let result = ('a--b--c' | split row '--')
  assert ($result == [a, b, c])
}

# This is the custom command 3 for split_row:

#[test]
def split_row_split_a_string_by___3 [] {
  let result = ('-a-b-c-' | split row '-')
  assert ($result == [, a, b, c, ])
}

# This is the custom command 4 for split_row:

#[test]
def split_row_split_a_string_by_regex_4 [] {
  let result = ('a   b       c' | split row -r '\s+')
  assert ($result == [a, b, c])
}


