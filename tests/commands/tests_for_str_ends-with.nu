use std assert

# Parameter name:
# sig type   : list<string>
# name       : string
# type       : positional
# shape      : string
# description: the string to match

# Parameter name:
# sig type   : list<string>
# name       : ignore-case
# type       : switch
# shape      : 
# description: search is case insensitive

# Parameter name:
# sig type   : record
# name       : string
# type       : positional
# shape      : string
# description: the string to match

# Parameter name:
# sig type   : record
# name       : ignore-case
# type       : switch
# shape      : 
# description: search is case insensitive

# Parameter name:
# sig type   : string
# name       : string
# type       : positional
# shape      : string
# description: the string to match

# Parameter name:
# sig type   : string
# name       : ignore-case
# type       : switch
# shape      : 
# description: search is case insensitive

# Parameter name:
# sig type   : table
# name       : string
# type       : positional
# shape      : string
# description: the string to match

# Parameter name:
# sig type   : table
# name       : ignore-case
# type       : switch
# shape      : 
# description: search is case insensitive


# This is the custom command 1 for str_ends-with:

#[test]
def str_ends-with_checks_if_string_ends_with_rb_1 [] {
  let result = ('my_library.rb' | str ends-with '.rb')
  assert ($result == true)
}

# This is the custom command 2 for str_ends-with:

#[test]
def str_ends-with_checks_if_strings_end_with_txt_2 [] {
  let result = (['my_library.rb', 'README.txt'] | str ends-with '.txt')
  assert ($result == [false, true])
}

# This is the custom command 3 for str_ends-with:

#[test]
def str_ends-with_checks_if_string_ends_with_rb_case_insensitive_3 [] {
  let result = ('my_library.rb' | str ends-with -i '.RB')
  assert ($result == true)
}


