use std assert

# Parameter name:
# sig type   : nothing
# name       : filename
# type       : positional
# shape      : path
# description: the path of the file you want to remove

# Parameter name:
# sig type   : nothing
# name       : trash
# type       : switch
# shape      : 
# description: move to the platform's trash instead of permanently deleting. not used on android and ios

# Parameter name:
# sig type   : nothing
# name       : permanent
# type       : switch
# shape      : 
# description: delete permanently, ignoring the 'always_trash' config option. always enabled on android and ios

# Parameter name:
# sig type   : nothing
# name       : recursive
# type       : switch
# shape      : 
# description: delete subdirectories recursively

# Parameter name:
# sig type   : nothing
# name       : force
# type       : switch
# shape      : 
# description: suppress error when no file

# Parameter name:
# sig type   : nothing
# name       : verbose
# type       : switch
# shape      : 
# description: print names of deleted files

# Parameter name:
# sig type   : nothing
# name       : interactive
# type       : switch
# shape      : 
# description: ask user to confirm action

# Parameter name:
# sig type   : nothing
# name       : interactive-once
# type       : switch
# shape      : 
# description: ask user to confirm action only once


# This is the custom command 1 for rm:

#[test]
def rm_delete_or_move_a_file_to_the_trash_based_on_the_always_trash_config_option_1 [] {
  let result = (rm file.txt)
  assert ($result == )
}

# This is the custom command 2 for rm:

#[test]
def rm_move_a_file_to_the_trash_2 [] {
  let result = (rm --trash file.txt)
  assert ($result == )
}

# This is the custom command 3 for rm:

#[test]
def rm_delete_a_file_permanently_even_if_the_always_trash_config_option_is_true_3 [] {
  let result = (rm --permanent file.txt)
  assert ($result == )
}

# This is the custom command 4 for rm:

#[test]
def rm_delete_a_file_ignoring_file_not_found_errors_4 [] {
  let result = (rm --force file.txt)
  assert ($result == )
}

# This is the custom command 5 for rm:

#[test]
def rm_delete_all_0kb_files_in_the_current_directory_5 [] {
  let result = (ls | where size == 0KB and type == file | each { rm $in.name } | null)
  assert ($result == )
}


