use std assert


# This is the custom command 1 for str_kebab-case:

#[test]
def str_kebab-case_convert_a_string_to_kebab_case_1 [] {
  let result = ('NuShell' | str kebab-case)
  assert ($result == nu-shell)
}

# This is the custom command 2 for str_kebab-case:

#[test]
def str_kebab-case_convert_a_string_to_kebab_case_2 [] {
  let result = ('thisIsTheFirstCase' | str kebab-case)
  assert ($result == this-is-the-first-case)
}

# This is the custom command 3 for str_kebab-case:

#[test]
def str_kebab-case_convert_a_string_to_kebab_case_3 [] {
  let result = ('THIS_IS_THE_SECOND_CASE' | str kebab-case)
  assert ($result == this-is-the-second-case)
}

# This is the custom command 4 for str_kebab-case:

#[test]
def str_kebab-case_convert_a_column_from_a_table_to_kebab_case_4 [] {
  let result = ([[lang, gems]; [nuTest, 100]] | str kebab-case lang)
  assert ($result == [{lang: nu-test, gems: 100}])
}


