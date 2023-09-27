use std assert


# This is the custom command 1 for path_type:

#[test]
def path_type_show_type_of_a_filepath_1 [] {
  let result = ('.' | path type)
  assert ($result == dir)
}

# This is the custom command 2 for path_type:

#[test]
def path_type_show_type_of_a_filepaths_in_a_list_2 [] {
  let result = (ls | get name | path type)
  assert ($result == )
}


