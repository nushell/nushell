use std assert


# This is the custom command 1 for nu-highlight:

#[test]
def nu-highlight_describe_the_type_of_a_string_1 [] {
  let result = ('let x = 3' | nu-highlight)
  assert ($result == )
}


