use std assert


# This is the custom command 1 for str_screaming-snake-case:

#[test]
def str_screaming-snake-case_convert_a_string_to_screaming_snake_case_1 [] {
  let result = ( "NuShell" | str screaming-snake-case)
  assert ($result == NU_SHELL)
}

# This is the custom command 2 for str_screaming-snake-case:

#[test]
def str_screaming-snake-case_convert_a_string_to_screaming_snake_case_2 [] {
  let result = ( "this_is_the_second_case" | str screaming-snake-case)
  assert ($result == THIS_IS_THE_SECOND_CASE)
}

# This is the custom command 3 for str_screaming-snake-case:

#[test]
def str_screaming-snake-case_convert_a_string_to_screaming_snake_case_3 [] {
  let result = ("this-is-the-first-case" | str screaming-snake-case)
  assert ($result == THIS_IS_THE_FIRST_CASE)
}

# This is the custom command 4 for str_screaming-snake-case:

#[test]
def str_screaming-snake-case_convert_a_column_from_a_table_to_screaming_snake_case_4 [] {
  let result = ([[lang, gems]; [nu_test, 100]] | str screaming-snake-case lang)
  assert ($result == [{lang: NU_TEST, gems: 100}])
}


