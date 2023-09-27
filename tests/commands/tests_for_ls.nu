use std assert

# Parameter name:
# sig type   : nothing
# name       : pattern
# type       : positional
# shape      : string
# description: the glob pattern to use

# Parameter name:
# sig type   : nothing
# name       : all
# type       : switch
# shape      : 
# description: Show hidden files

# Parameter name:
# sig type   : nothing
# name       : long
# type       : switch
# shape      : 
# description: Get all available columns for each entry (slower; columns are platform-dependent)

# Parameter name:
# sig type   : nothing
# name       : short-names
# type       : switch
# shape      : 
# description: Only print the file names, and not the path

# Parameter name:
# sig type   : nothing
# name       : full-paths
# type       : switch
# shape      : 
# description: display paths as absolute paths

# Parameter name:
# sig type   : nothing
# name       : du
# type       : switch
# shape      : 
# description: Display the apparent directory size ("disk usage") in place of the directory metadata size

# Parameter name:
# sig type   : nothing
# name       : directory
# type       : switch
# shape      : 
# description: List the specified directory itself instead of its contents

# Parameter name:
# sig type   : nothing
# name       : mime-type
# type       : switch
# shape      : 
# description: Show mime-type in type column instead of 'file' (based on filenames only; files' contents are not examined)


# This is the custom command 1 for ls:

#[test]
def ls_list_visible_files_in_the_current_directory_1 [] {
  let result = (ls)
  assert ($result == )
}

# This is the custom command 2 for ls:

#[test]
def ls_list_visible_files_in_a_subdirectory_2 [] {
  let result = (ls subdir)
  assert ($result == )
}

# This is the custom command 3 for ls:

#[test]
def ls_list_visible_files_with_full_path_in_the_parent_directory_3 [] {
  let result = (ls -f ..)
  assert ($result == )
}

# This is the custom command 4 for ls:

#[test]
def ls_list_rust_files_4 [] {
  let result = (ls *.rs)
  assert ($result == )
}

# This is the custom command 5 for ls:

#[test]
def ls_list_files_and_directories_whose_name_do_not_contain_bar_5 [] {
  let result = (ls -s | where name !~ bar)
  assert ($result == )
}

# This is the custom command 6 for ls:

#[test]
def ls_list_all_dirs_in_your_home_directory_6 [] {
  let result = (ls -a ~ | where type == dir)
  assert ($result == )
}

# This is the custom command 7 for ls:

#[test]
def ls_list_all_dirs_in_your_home_directory_which_have_not_been_modified_in_7_days_7 [] {
  let result = (ls -as ~ | where type == dir and modified < ((date now) - 7day))
  assert ($result == )
}

# This is the custom command 8 for ls:

#[test]
def ls_list_given_paths_and_show_directories_themselves_8 [] {
  let result = (['/path/to/directory' '/path/to/file'] | each {|| ls -D $in } | flatten)
  assert ($result == )
}


