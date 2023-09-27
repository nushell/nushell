use std assert


# This is the custom command 1 for str_snake-case:

#[test]
def str_snake-case_convert_a_string_to_snake_case_1 [] {
  let result = ( "NuShell" | str snake-case)
  assert ($result == nu_shell)
}

# This is the custom command 2 for str_snake-case:

#[test]
def str_snake-case_convert_a_string_to_snake_case_2 [] {
  let result = ( "this_is_the_second_case" | str snake-case)
  assert ($result == this_is_the_second_case)
}

# This is the custom command 3 for str_snake-case:

#[test]
def str_snake-case_convert_a_string_to_snake_case_3 [] {
  let result = ("this-is-the-first-case" | str snake-case)
  assert ($result == this_is_the_first_case)
}

# This is the custom command 4 for str_snake-case:

#[test]
def str_snake-case_convert_a_column_from_a_table_to_snake_case_4 [] {
  let result = ([[lang, gems]; [nuTest, 100]] | str snake-case lang)
  assert ($result == [{lang: nu_test, gems: 100}])
}


