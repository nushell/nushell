use std assert


# This is the custom command 1 for str_camel-case:

#[test]
def str_camel-case_convert_a_string_to_camelcase_1 [] {
  let result = ( 'NuShell' | str camel-case)
  assert ($result == nuShell)
}

# This is the custom command 2 for str_camel-case:

#[test]
def str_camel-case_convert_a_string_to_camelcase_2 [] {
  let result = ('this-is-the-first-case' | str camel-case)
  assert ($result == thisIsTheFirstCase)
}

# This is the custom command 3 for str_camel-case:

#[test]
def str_camel-case_convert_a_string_to_camelcase_3 [] {
  let result = ( 'this_is_the_second_case' | str camel-case)
  assert ($result == thisIsTheSecondCase)
}

# This is the custom command 4 for str_camel-case:

#[test]
def str_camel-case_convert_a_column_from_a_table_to_camelcase_4 [] {
  let result = ([[lang, gems]; [nu_test, 100]] | str camel-case lang)
  assert ($result == [{lang: nuTest, gems: 100}])
}


