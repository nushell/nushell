use std assert


# This is the custom command 1 for path_exists:

#[test]
def path_exists_check_if_a_file_exists_1 [] {
  let result = ('C:\Users\joe\todo.txt' | path exists)
  assert ($result == false)
}

# This is the custom command 2 for path_exists:

#[test]
def path_exists_check_if_files_in_list_exist_2 [] {
  let result = ([ C:\joe\todo.txt, C:\Users\doe\todo.txt ] | path exists)
  assert ($result == [false, false])
}


