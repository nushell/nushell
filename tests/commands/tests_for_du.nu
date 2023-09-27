use std assert

# Parameter name:
# sig type   : nothing
# name       : path
# type       : positional
# shape      : glob
# description: starting directory

# Parameter name:
# sig type   : nothing
# name       : all
# type       : switch
# shape      : 
# description: Output file sizes as well as directory sizes

# Parameter name:
# sig type   : nothing
# name       : deref
# type       : switch
# shape      : 
# description: Dereference symlinks to their targets for size

# Parameter name:
# sig type   : nothing
# name       : exclude
# type       : named
# shape      : glob
# description: Exclude these file names

# Parameter name:
# sig type   : nothing
# name       : max-depth
# type       : named
# shape      : int
# description: Directory recursion limit

# Parameter name:
# sig type   : nothing
# name       : min-size
# type       : named
# shape      : int
# description: Exclude files below this size


# This is the custom command 1 for du:

#[test]
def du_disk_usage_of_the_current_directory_1 [] {
  let result = (du)
  assert ($result == )
}


