use std assert

# Parameter name:
# sig type   : list<string>
# name       : string
# type       : positional
# shape      : string
# description: the substring to find

# Parameter name:
# sig type   : list<string>
# name       : ignore-case
# type       : switch
# shape      : 
# description: search is case insensitive

# Parameter name:
# sig type   : list<string>
# name       : not
# type       : switch
# shape      : 
# description: does not contain

# Parameter name:
# sig type   : record
# name       : string
# type       : positional
# shape      : string
# description: the substring to find

# Parameter name:
# sig type   : record
# name       : ignore-case
# type       : switch
# shape      : 
# description: search is case insensitive

# Parameter name:
# sig type   : record
# name       : not
# type       : switch
# shape      : 
# description: does not contain

# Parameter name:
# sig type   : string
# name       : string
# type       : positional
# shape      : string
# description: the substring to find

# Parameter name:
# sig type   : string
# name       : ignore-case
# type       : switch
# shape      : 
# description: search is case insensitive

# Parameter name:
# sig type   : string
# name       : not
# type       : switch
# shape      : 
# description: does not contain

# Parameter name:
# sig type   : table
# name       : string
# type       : positional
# shape      : string
# description: the substring to find

# Parameter name:
# sig type   : table
# name       : ignore-case
# type       : switch
# shape      : 
# description: search is case insensitive

# Parameter name:
# sig type   : table
# name       : not
# type       : switch
# shape      : 
# description: does not contain


# This is the custom command 1 for str_contains:

#[test]
def str_contains_check_if_input_contains_string_1 [] {
  let result = ('my_library.rb' | str contains '.rb')
  assert ($result == true)
}

# This is the custom command 2 for str_contains:

#[test]
def str_contains_check_if_input_contains_string_case_insensitive_2 [] {
  let result = ('my_library.rb' | str contains -i '.RB')
  assert ($result == true)
}

# This is the custom command 3 for str_contains:

#[test]
def str_contains_check_if_input_contains_string_in_a_record_3 [] {
  let result = ({ ColA: test, ColB: 100 } | str contains 'e' ColA)
  assert ($result == {ColA: true, ColB: 100})
}

# This is the custom command 4 for str_contains:

#[test]
def str_contains_check_if_input_contains_string_in_a_table_4 [] {
  let result = ( [[ColA ColB]; [test 100]] | str contains -i 'E' ColA)
  assert ($result == [{ColA: true, ColB: 100}])
}

# This is the custom command 5 for str_contains:

#[test]
def str_contains_check_if_input_contains_string_in_a_table_5 [] {
  let result = ( [[ColA ColB]; [test hello]] | str contains 'e' ColA ColB)
  assert ($result == [{ColA: true, ColB: true}])
}

# This is the custom command 6 for str_contains:

#[test]
def str_contains_check_if_input_string_contains_banana_6 [] {
  let result = ('hello' | str contains 'banana')
  assert ($result == false)
}

# This is the custom command 7 for str_contains:

#[test]
def str_contains_check_if_list_contains_string_7 [] {
  let result = ([one two three] | str contains o)
  assert ($result == [true, true, false])
}

# This is the custom command 8 for str_contains:

#[test]
def str_contains_check_if_list_does_not_contain_string_8 [] {
  let result = ([one two three] | str contains -n o)
  assert ($result == [false, false, true])
}


