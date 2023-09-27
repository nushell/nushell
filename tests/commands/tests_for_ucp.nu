use std assert

# Parameter name:
# sig type   : nothing
# name       : paths
# type       : rest
# shape      : path
# description: Copy SRC file/s to DEST

# Parameter name:
# sig type   : nothing
# name       : recursive
# type       : switch
# shape      : 
# description: copy directories recursively

# Parameter name:
# sig type   : nothing
# name       : verbose
# type       : switch
# shape      : 
# description: explicitly state what is being done

# Parameter name:
# sig type   : nothing
# name       : force
# type       : switch
# shape      : 
# description: if an existing destination file cannot be opened, remove it and try                     again (this option is ignored when the -n option is also used).                     currently not implemented for windows

# Parameter name:
# sig type   : nothing
# name       : interactive
# type       : switch
# shape      : 
# description: ask before overwriting files

# Parameter name:
# sig type   : nothing
# name       : progress
# type       : switch
# shape      : 
# description: display a progress bar

# Parameter name:
# sig type   : nothing
# name       : no-clobber
# type       : switch
# shape      : 
# description: do not overwrite an existing file

# Parameter name:
# sig type   : nothing
# name       : debug
# type       : switch
# shape      : 
# description: explain how a file is copied. Implies -v


# This is the custom command 1 for ucp:

#[test]
def ucp_copy_myfile_to_dir_b_1 [] {
  let result = (ucp myfile dir_b)
  assert ($result == )
}

# This is the custom command 2 for ucp:

#[test]
def ucp_recursively_copy_dir_a_to_dir_b_2 [] {
  let result = (ucp -r dir_a dir_b)
  assert ($result == )
}

# This is the custom command 3 for ucp:

#[test]
def ucp_recursively_copy_dir_a_to_dir_b_and_print_the_feedbacks_3 [] {
  let result = (ucp -r -v dir_a dir_b)
  assert ($result == )
}

# This is the custom command 4 for ucp:

#[test]
def ucp_move_many_files_into_a_directory_4 [] {
  let result = (ucp *.txt dir_a)
  assert ($result == )
}


