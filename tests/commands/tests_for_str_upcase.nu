use std assert


# This is the custom command 1 for str_upcase:

#[test]
def str_upcase_upcase_contents_1 [] {
  let result = ('nu' | str upcase)
  assert ($result == NU)
}


