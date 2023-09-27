use std assert

# Parameter name:
# sig type   : nothing
# name       : source
# type       : positional
# shape      : glob
# description: the place to copy from

# Parameter name:
# sig type   : nothing
# name       : destination
# type       : positional
# shape      : path
# description: the place to copy to

# Parameter name:
# sig type   : nothing
# name       : recursive
# type       : switch
# shape      : 
# description: copy recursively through subdirectories

# Parameter name:
# sig type   : nothing
# name       : verbose
# type       : switch
# shape      : 
# description: show successful copies in addition to failed copies (default:false)

# Parameter name:
# sig type   : nothing
# name       : update
# type       : switch
# shape      : 
# description: copy only when the SOURCE file is newer than the destination file or when the destination file is missing

# Parameter name:
# sig type   : nothing
# name       : interactive
# type       : switch
# shape      : 
# description: ask user to confirm action

# Parameter name:
# sig type   : nothing
# name       : no-symlink
# type       : switch
# shape      : 
# description: no symbolic links are followed, only works if -r is active

# Parameter name:
# sig type   : nothing
# name       : progress
# type       : switch
# shape      : 
# description: enable progress bar


# This is the custom command 1 for cp:

#[test]
def cp_copy_myfile_to_dir_b_1 [] {
  let result = (cp myfile dir_b)
  assert ($result == )
}

# This is the custom command 2 for cp:

#[test]
def cp_recursively_copy_dir_a_to_dir_b_2 [] {
  let result = (cp -r dir_a dir_b)
  assert ($result == )
}

# This is the custom command 3 for cp:

#[test]
def cp_recursively_copy_dir_a_to_dir_b_and_print_the_feedbacks_3 [] {
  let result = (cp -r -v dir_a dir_b)
  assert ($result == )
}

# This is the custom command 4 for cp:

#[test]
def cp_move_many_files_into_a_directory_4 [] {
  let result = (cp *.txt dir_a)
  assert ($result == )
}

# This is the custom command 5 for cp:

#[test]
def cp_copy_only_if_source_file_is_newer_than_target_file_5 [] {
  let result = (cp -u a b)
  assert ($result == )
}


