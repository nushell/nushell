use std assert

# Parameter name:
# sig type   : nothing
# name       : verbose
# type       : switch
# shape      : 
# description: print created path(s).


# This is the custom command 1 for mkdir:

#[test]
def mkdir_make_a_directory_named_foo_1 [] {
  let result = (mkdir foo)
  assert ($result == )
}

# This is the custom command 2 for mkdir:

#[test]
def mkdir_make_multiple_directories_and_show_the_paths_created_2 [] {
  let result = (mkdir -v foo/bar foo2)
  assert ($result == )
}


