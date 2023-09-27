use std assert

# Parameter name:
# sig type   : nothing
# name       : source
# type       : positional
# shape      : glob
# description: the location to move files/directories from

# Parameter name:
# sig type   : nothing
# name       : destination
# type       : positional
# shape      : path
# description: the location to move files/directories to

# Parameter name:
# sig type   : nothing
# name       : verbose
# type       : switch
# shape      : 
# description: make mv to be verbose, showing files been moved.

# Parameter name:
# sig type   : nothing
# name       : force
# type       : switch
# shape      : 
# description: overwrite the destination.

# Parameter name:
# sig type   : nothing
# name       : interactive
# type       : switch
# shape      : 
# description: ask user to confirm action

# Parameter name:
# sig type   : nothing
# name       : update
# type       : switch
# shape      : 
# description: move only when the SOURCE file is newer than the destination file(with -f) or when the destination file is missing


# This is the custom command 1 for mv:

#[test]
def mv_rename_a_file_1 [] {
  let result = (mv before.txt after.txt)
  assert ($result == )
}

# This is the custom command 2 for mv:

#[test]
def mv_move_a_file_into_a_directory_2 [] {
  let result = (mv test.txt my/subdirectory)
  assert ($result == )
}

# This is the custom command 3 for mv:

#[test]
def mv_move_many_files_into_a_directory_3 [] {
  let result = (mv *.txt my/subdirectory)
  assert ($result == )
}


