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


# This is the custom command 1 for str_starts-with:

#[test]
def str_starts-with_checks_if_input_string_starts_with_my_1 [] {
  let result = ('my_library.rb' | str starts-with 'my')
  assert ($result == true)
}

# This is the custom command 2 for str_starts-with:

#[test]
def str_starts-with_checks_if_input_string_starts_with_car_2 [] {
  let result = ('Cargo.toml' | str starts-with 'Car')
  assert ($result == true)
}

# This is the custom command 3 for str_starts-with:

#[test]
def str_starts-with_checks_if_input_string_starts_with_toml_3 [] {
  let result = ('Cargo.toml' | str starts-with '.toml')
  assert ($result == false)
}

# This is the custom command 4 for str_starts-with:

#[test]
def str_starts-with_checks_if_input_string_starts_with_cargo_case_insensitive_4 [] {
  let result = ('Cargo.toml' | str starts-with -i 'cargo')
  assert ($result == true)
}


