use std assert

# Parameter name:
# sig type   : any
# name       : head
# type       : named
# shape      : bool
# description: Show or hide column headers (default true)

# Parameter name:
# sig type   : any
# name       : index
# type       : switch
# shape      : 
# description: Show row indexes when viewing a list

# Parameter name:
# sig type   : any
# name       : reverse
# type       : switch
# shape      : 
# description: Start with the viewport scrolled to the bottom

# Parameter name:
# sig type   : any
# name       : peek
# type       : switch
# shape      : 
# description: When quitting, output the value of the cell the cursor was on


# This is the custom command 1 for explore:

#[test]
def explore_explore_the_system_information_record_1 [] {
  let result = (sys | explore)
  assert ($result == )
}

# This is the custom command 2 for explore:

#[test]
def explore_explore_the_output_of_ls_without_column_names_2 [] {
  let result = (ls | explore --head false)
  assert ($result == )
}

# This is the custom command 3 for explore:

#[test]
def explore_explore_a_list_of_markdown_files_contents_with_row_indexes_3 [] {
  let result = (glob *.md | each {|| open } | explore -i)
  assert ($result == )
}

# This is the custom command 4 for explore:

#[test]
def explore_explore_a_json_file_then_save_the_last_visited_sub_structure_to_a_file_4 [] {
  let result = (open file.json | explore -p | to json | save part.json)
  assert ($result == )
}


