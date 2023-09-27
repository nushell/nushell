use std assert

# Parameter name:
# sig type   : any
# name       : default value
# type       : positional
# shape      : any
# description: the value to use as a default

# Parameter name:
# sig type   : any
# name       : column name
# type       : positional
# shape      : string
# description: the name of the column


# This is the custom command 1 for default:

#[test]
def default_give_a_default_target_column_to_all_file_entries_1 [] {
  let result = (ls -la | default 'nothing' target )
  assert ($result == )
}

# This is the custom command 2 for default:

#[test]
def default_get_the_env_value_of_my_env_with_a_default_value_abc_if_not_present_2 [] {
  let result = ($env | get -i MY_ENV | default 'abc')
  assert ($result == )
}

# This is the custom command 3 for default:

#[test]
def default_replace_the_null_value_in_a_list_3 [] {
  let result = ([1, 2, null, 4] | default 3)
  assert ($result == [1, 2, 3, 4])
}


