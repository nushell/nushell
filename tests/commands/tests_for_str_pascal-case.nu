use std assert


# This is the custom command 1 for str_pascal-case:

#[test]
def str_pascal-case_convert_a_string_to_pascalcase_1 [] {
  let result = ('nu-shell' | str pascal-case)
  assert ($result == NuShell)
}

# This is the custom command 2 for str_pascal-case:

#[test]
def str_pascal-case_convert_a_string_to_pascalcase_2 [] {
  let result = ('this-is-the-first-case' | str pascal-case)
  assert ($result == ThisIsTheFirstCase)
}

# This is the custom command 3 for str_pascal-case:

#[test]
def str_pascal-case_convert_a_string_to_pascalcase_3 [] {
  let result = ('this_is_the_second_case' | str pascal-case)
  assert ($result == ThisIsTheSecondCase)
}

# This is the custom command 4 for str_pascal-case:

#[test]
def str_pascal-case_convert_a_column_from_a_table_to_pascalcase_4 [] {
  let result = ([[lang, gems]; [nu_test, 100]] | str pascal-case lang)
  assert ($result == [{lang: NuTest, gems: 100}])
}


