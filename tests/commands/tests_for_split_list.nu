use std assert

# Parameter name:
# sig type   : list<any>
# name       : separator
# type       : positional
# shape      : any
# description: the value that denotes what separates the list

# Parameter name:
# sig type   : list<any>
# name       : regex
# type       : switch
# shape      : 
# description: separator is a regular expression, matching values that can be coerced into a string


# This is the custom command 1 for split_list:

#[test]
def split_list_split_a_list_of_chars_into_two_lists_1 [] {
  let result = ([a, b, c, d, e, f, g] | split list d)
  assert ($result == [[a, b, c], [e, f, g]])
}

# This is the custom command 2 for split_list:

#[test]
def split_list_split_a_list_of_lists_into_two_lists_of_lists_2 [] {
  let result = ([[1,2], [2,3], [3,4]] | split list [2,3])
  assert ($result == [[[1, 2]], [[3, 4]]])
}

# This is the custom command 3 for split_list:

#[test]
def split_list_split_a_list_of_chars_into_two_lists_3 [] {
  let result = ([a, b, c, d, a, e, f, g] | split list a)
  assert ($result == [[b, c, d], [e, f, g]])
}

# This is the custom command 4 for split_list:

#[test]
def split_list_split_a_list_of_chars_into_lists_based_on_multiple_characters_4 [] {
  let result = ([a, b, c, d, a, e, f, g] | split list -r '(b|e)')
  assert ($result == [[a], [c, d, a], [f, g]])
}


