use std assert


# This is the custom command 1 for str_title-case:

#[test]
def str_title-case_convert_a_string_to_title_case_1 [] {
  let result = ('nu-shell' | str title-case)
  assert ($result == Nu Shell)
}

# This is the custom command 2 for str_title-case:

#[test]
def str_title-case_convert_a_string_to_title_case_2 [] {
  let result = ('this is a test case' | str title-case)
  assert ($result == This Is A Test Case)
}

# This is the custom command 3 for str_title-case:

#[test]
def str_title-case_convert_a_column_from_a_table_to_title_case_3 [] {
  let result = ([[title, count]; ['nu test', 100]] | str title-case title)
  assert ($result == [{title: Nu Test, count: 100}])
}


