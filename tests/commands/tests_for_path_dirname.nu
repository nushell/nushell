use std assert

# Parameter name:
# sig type   : list<string>
# name       : replace
# type       : named
# shape      : string
# description: Return original path with dirname replaced by this string

# Parameter name:
# sig type   : list<string>
# name       : num-levels
# type       : named
# shape      : int
# description: Number of directories to walk up

# Parameter name:
# sig type   : string
# name       : replace
# type       : named
# shape      : string
# description: Return original path with dirname replaced by this string

# Parameter name:
# sig type   : string
# name       : num-levels
# type       : named
# shape      : int
# description: Number of directories to walk up


# This is the custom command 1 for path_dirname:

#[test]
def path_dirname_get_dirname_of_a_path_1 [] {
  let result = ('C:\Users\joe\code\test.txt' | path dirname)
  assert ($result == C:\Users\joe\code)
}

# This is the custom command 2 for path_dirname:

#[test]
def path_dirname_get_dirname_of_a_list_of_paths_2 [] {
  let result = ([ C:\Users\joe\test.txt, C:\Users\doe\test.txt ] | path dirname)
  assert ($result == [C:\Users\joe, C:\Users\doe])
}

# This is the custom command 3 for path_dirname:

#[test]
def path_dirname_walk_up_two_levels_3 [] {
  let result = ('C:\Users\joe\code\test.txt' | path dirname -n 2)
  assert ($result == C:\Users\joe)
}

# This is the custom command 4 for path_dirname:

#[test]
def path_dirname_replace_the_part_that_would_be_returned_with_a_custom_path_4 [] {
  let result = ('C:\Users\joe\code\test.txt' | path dirname -n 2 -r C:\Users\viking)
  assert ($result == C:\Users\viking\code\test.txt)
}


