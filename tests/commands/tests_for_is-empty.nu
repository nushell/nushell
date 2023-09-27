use std assert


# This is the custom command 1 for is-empty:

#[test]
def is-empty_check_if_a_string_is_empty_1 [] {
  let result = ('' | is-empty)
  assert ($result == true)
}

# This is the custom command 2 for is-empty:

#[test]
def is-empty_check_if_a_list_is_empty_2 [] {
  let result = ([] | is-empty)
  assert ($result == true)
}

# This is the custom command 3 for is-empty:

#[test]
def is-empty_check_if_more_than_one_column_are_empty_3 [] {
  let result = ([[meal size]; [arepa small] [taco '']] | is-empty meal size)
  assert ($result == false)
}


