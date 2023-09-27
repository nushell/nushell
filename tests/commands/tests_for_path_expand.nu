use std assert

# Parameter name:
# sig type   : list<string>
# name       : strict
# type       : switch
# shape      : 
# description: Throw an error if the path could not be expanded

# Parameter name:
# sig type   : list<string>
# name       : no-symlink
# type       : switch
# shape      : 
# description: Do not resolve symbolic links

# Parameter name:
# sig type   : string
# name       : strict
# type       : switch
# shape      : 
# description: Throw an error if the path could not be expanded

# Parameter name:
# sig type   : string
# name       : no-symlink
# type       : switch
# shape      : 
# description: Do not resolve symbolic links


# This is the custom command 1 for path_expand:

#[test]
def path_expand_expand_an_absolute_path_1 [] {
  let result = ('C:\Users\joe\foo\..\bar' | path expand)
  assert ($result == C:\Users\joe\bar)
}

# This is the custom command 2 for path_expand:

#[test]
def path_expand_expand_a_relative_path_2 [] {
  let result = ('foo\..\bar' | path expand)
  assert ($result == )
}

# This is the custom command 3 for path_expand:

#[test]
def path_expand_expand_a_list_of_paths_3 [] {
  let result = ([ C:\foo\..\bar, C:\foo\..\baz ] | path expand)
  assert ($result == [C:\bar, C:\baz])
}


