use std assert

# Parameter name:
# sig type   : list<string>
# name       : path
# type       : positional
# shape      : string
# description: Parent shared with the input path

# Parameter name:
# sig type   : string
# name       : path
# type       : positional
# shape      : string
# description: Parent shared with the input path


# This is the custom command 1 for path_relative-to:

#[test]
def path_relative-to_find_a_relative_path_from_two_absolute_paths_1 [] {
  let result = ('C:\Users\viking' | path relative-to 'C:\Users')
  assert ($result == viking)
}

# This is the custom command 2 for path_relative-to:

#[test]
def path_relative-to_find_a_relative_path_from_absolute_paths_in_list_2 [] {
  let result = ([ C:\Users\viking, C:\Users\spam ] | path relative-to C:\Users)
  assert ($result == [viking, spam])
}

# This is the custom command 3 for path_relative-to:

#[test]
def path_relative-to_find_a_relative_path_from_two_relative_paths_3 [] {
  let result = ('eggs\bacon\sausage\spam' | path relative-to 'eggs\bacon\sausage')
  assert ($result == spam)
}


