use std assert

# Parameter name:
# sig type   : nothing
# name       : glob
# type       : positional
# shape      : string
# description: the glob expression

# Parameter name:
# sig type   : nothing
# name       : depth
# type       : named
# shape      : int
# description: directory depth to search

# Parameter name:
# sig type   : nothing
# name       : no-dir
# type       : switch
# shape      : 
# description: Whether to filter out directories from the returned paths

# Parameter name:
# sig type   : nothing
# name       : no-file
# type       : switch
# shape      : 
# description: Whether to filter out files from the returned paths

# Parameter name:
# sig type   : nothing
# name       : no-symlink
# type       : switch
# shape      : 
# description: Whether to filter out symlinks from the returned paths

# Parameter name:
# sig type   : nothing
# name       : not
# type       : named
# shape      : list<string>
# description: Patterns to exclude from the results


# This is the custom command 1 for glob:

#[test]
def glob_search_for_rs_files_1 [] {
  let result = (glob *.rs)
  assert ($result == )
}

# This is the custom command 2 for glob:

#[test]
def glob_search_for_rs_and_toml_files_recursively_up_to_2_folders_deep_2 [] {
  let result = (glob **/*.{rs,toml} --depth 2)
  assert ($result == )
}

# This is the custom command 3 for glob:

#[test]
def glob_search_for_files_and_folders_that_begin_with_uppercase_c_and_lowercase_c_3 [] {
  let result = (glob "[Cc]*")
  assert ($result == )
}

# This is the custom command 4 for glob:

#[test]
def glob_search_for_files_and_folders_like_abc_or_xyz_substituting_a_character_for__4 [] {
  let result = (glob "{a?c,x?z}")
  assert ($result == )
}

# This is the custom command 5 for glob:

#[test]
def glob_a_case_insensitive_search_for_files_and_folders_that_begin_with_c_5 [] {
  let result = (glob "(?i)c*")
  assert ($result == )
}

# This is the custom command 6 for glob:

#[test]
def glob_search_for_files_for_folders_that_do_not_begin_with_c_c_b_m_or_s_6 [] {
  let result = (glob "[!cCbMs]*")
  assert ($result == )
}

# This is the custom command 7 for glob:

#[test]
def glob_search_for_files_or_folders_with_3_as_in_a_row_in_the_name_7 [] {
  let result = (glob <a*:3>)
  assert ($result == )
}

# This is the custom command 8 for glob:

#[test]
def glob_search_for_files_or_folders_with_only_a_b_c_or_d_in_the_file_name_between_1_and_10_times_8 [] {
  let result = (glob <[a-d]:1,10>)
  assert ($result == )
}

# This is the custom command 9 for glob:

#[test]
def glob_search_for_folders_that_begin_with_an_uppercase_ascii_letter_ignoring_files_and_symlinks_9 [] {
  let result = (glob "[A-Z]*" --no-file --no-symlink)
  assert ($result == )
}

# This is the custom command 10 for glob:

#[test]
def glob_search_for_files_named_tsconfigjson_that_are_not_in_node_modules_directories_10 [] {
  let result = (glob **/tsconfig.json --not [**/node_modules/**])
  assert ($result == )
}

# This is the custom command 11 for glob:

#[test]
def glob_search_for_all_files_that_are_not_in_the_target_nor_git_directories_11 [] {
  let result = (glob **/* --not [**/target/** **/.git/** */])
  assert ($result == )
}


