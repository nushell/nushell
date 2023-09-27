use std assert

# Parameter name:
# sig type   : list<string>
# name       : append
# type       : rest
# shape      : string
# description: Path to append to the input

# Parameter name:
# sig type   : record
# name       : append
# type       : rest
# shape      : string
# description: Path to append to the input

# Parameter name:
# sig type   : string
# name       : append
# type       : rest
# shape      : string
# description: Path to append to the input

# Parameter name:
# sig type   : table
# name       : append
# type       : rest
# shape      : string
# description: Path to append to the input


# This is the custom command 1 for path_join:

#[test]
def path_join_append_a_filename_to_a_path_1 [] {
  let result = ('C:\Users\viking' | path join spam.txt)
  assert ($result == C:\Users\viking\spam.txt)
}

# This is the custom command 2 for path_join:

#[test]
def path_join_append_a_filename_to_a_path_2 [] {
  let result = ('C:\Users\viking' | path join spams this_spam.txt)
  assert ($result == C:\Users\viking\spams\this_spam.txt)
}

# This is the custom command 3 for path_join:

#[test]
def path_join_join_a_list_of_parts_into_a_path_3 [] {
  let result = ([ 'C:' '\' 'Users' 'viking' 'spam.txt' ] | path join)
  assert ($result == C:\Users\viking\spam.txt)
}

# This is the custom command 4 for path_join:

#[test]
def path_join_join_a_structured_path_into_a_path_4 [] {
  let result = ({ parent: 'C:\Users\viking', stem: 'spam', extension: 'txt' } | path join)
  assert ($result == C:\Users\viking\spam.txt)
}

# This is the custom command 5 for path_join:

#[test]
def path_join_join_a_table_of_structured_paths_into_a_list_of_paths_5 [] {
  let result = ([ [parent stem extension]; ['C:\Users\viking' 'spam' 'txt']] | path join)
  assert ($result == [C:\Users\viking\spam.txt])
}


