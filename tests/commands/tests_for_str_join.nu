use std assert

# Parameter name:
# sig type   : list<any>
# name       : separator
# type       : positional
# shape      : string
# description: optional separator to use when creating string

# Parameter name:
# sig type   : string
# name       : separator
# type       : positional
# shape      : string
# description: optional separator to use when creating string


# This is the custom command 1 for str_join:

#[test]
def str_join_create_a_string_from_input_1 [] {
  let result = (['nu', 'shell'] | str join)
  assert ($result == nushell)
}

# This is the custom command 2 for str_join:

#[test]
def str_join_create_a_string_from_input_with_a_separator_2 [] {
  let result = (['nu', 'shell'] | str join '-')
  assert ($result == nu-shell)
}


