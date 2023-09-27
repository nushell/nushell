use std assert

# Parameter name:
# sig type   : list<string>
# name       : replace
# type       : named
# shape      : string
# description: Return original path with basename replaced by this string

# Parameter name:
# sig type   : string
# name       : replace
# type       : named
# shape      : string
# description: Return original path with basename replaced by this string


# This is the custom command 1 for path_basename:

#[test]
def path_basename_get_basename_of_a_path_1 [] {
  let result = ('C:\Users\joe\test.txt' | path basename)
  assert ($result == test.txt)
}

# This is the custom command 2 for path_basename:

#[test]
def path_basename_get_basename_of_a_list_of_paths_2 [] {
  let result = ([ C:\Users\joe, C:\Users\doe ] | path basename)
  assert ($result == [joe, doe])
}

# This is the custom command 3 for path_basename:

#[test]
def path_basename_replace_basename_of_a_path_3 [] {
  let result = ('C:\Users\joe\test.txt' | path basename -r 'spam.png')
  assert ($result == C:\Users\joe\spam.png)
}


