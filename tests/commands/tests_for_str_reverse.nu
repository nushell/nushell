use std assert


# This is the custom command 1 for str_reverse:

#[test]
def str_reverse_reverse_a_single_string_1 [] {
  let result = ('Nushell' | str reverse)
  assert ($result == llehsuN)
}

# This is the custom command 2 for str_reverse:

#[test]
def str_reverse_reverse_multiple_strings_in_a_list_2 [] {
  let result = (['Nushell' 'is' 'cool'] | str reverse)
  assert ($result == [llehsuN, si, looc])
}


