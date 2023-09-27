use std assert


# This is the custom command 1 for str_capitalize:

#[test]
def str_capitalize_capitalize_contents_1 [] {
  let result = ('good day' | str capitalize)
  assert ($result == Good day)
}

# This is the custom command 2 for str_capitalize:

#[test]
def str_capitalize_capitalize_contents_2 [] {
  let result = ('anton' | str capitalize)
  assert ($result == Anton)
}

# This is the custom command 3 for str_capitalize:

#[test]
def str_capitalize_capitalize_a_column_in_a_table_3 [] {
  let result = ([[lang, gems]; [nu_test, 100]] | str capitalize lang)
  assert ($result == [{lang: Nu_test, gems: 100}])
}


