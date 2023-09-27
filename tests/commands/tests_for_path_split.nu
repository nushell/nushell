use std assert


# This is the custom command 1 for path_split:

#[test]
def path_split_split_a_path_into_parts_1 [] {
  let result = ('C:\Users\viking\spam.txt' | path split)
  assert ($result == [C:\, Users, viking, spam.txt])
}

# This is the custom command 2 for path_split:

#[test]
def path_split_split_paths_in_list_into_parts_2 [] {
  let result = ([ C:\Users\viking\spam.txt C:\Users\viking\eggs.txt ] | path split)
  assert ($result == [[C:\, Users, viking, spam.txt], [C:\, Users, viking, eggs.txt]])
}


