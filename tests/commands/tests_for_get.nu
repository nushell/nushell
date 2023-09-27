use std assert

# Parameter name:
# sig type   : list<any>
# name       : cell_path
# type       : positional
# shape      : cell-path
# description: the cell path to the data

# Parameter name:
# sig type   : list<any>
# name       : ignore-errors
# type       : switch
# shape      : 
# description: ignore missing data (make all cell path members optional)

# Parameter name:
# sig type   : list<any>
# name       : sensitive
# type       : switch
# shape      : 
# description: get path in a case sensitive manner

# Parameter name:
# sig type   : record
# name       : cell_path
# type       : positional
# shape      : cell-path
# description: the cell path to the data

# Parameter name:
# sig type   : record
# name       : ignore-errors
# type       : switch
# shape      : 
# description: ignore missing data (make all cell path members optional)

# Parameter name:
# sig type   : record
# name       : sensitive
# type       : switch
# shape      : 
# description: get path in a case sensitive manner

# Parameter name:
# sig type   : table
# name       : cell_path
# type       : positional
# shape      : cell-path
# description: the cell path to the data

# Parameter name:
# sig type   : table
# name       : ignore-errors
# type       : switch
# shape      : 
# description: ignore missing data (make all cell path members optional)

# Parameter name:
# sig type   : table
# name       : sensitive
# type       : switch
# shape      : 
# description: get path in a case sensitive manner


# This is the custom command 1 for get:

#[test]
def get_get_an_item_from_a_list_1 [] {
  let result = ([0 1 2] | get 1)
  assert ($result == 1)
}

# This is the custom command 2 for get:

#[test]
def get_get_a_column_from_a_table_2 [] {
  let result = ([{A: A0}] | get A)
  assert ($result == [A0])
}

# This is the custom command 3 for get:

#[test]
def get_get_a_cell_from_a_table_3 [] {
  let result = ([{A: A0}] | get 0.A)
  assert ($result == A0)
}

# This is the custom command 4 for get:

#[test]
def get_extract_the_name_of_the_3rd_record_in_a_list_same_as_ls__inname_4 [] {
  let result = (ls | get name.2)
  assert ($result == )
}

# This is the custom command 5 for get:

#[test]
def get_extract_the_name_of_the_3rd_record_in_a_list_5 [] {
  let result = (ls | get 2.name)
  assert ($result == )
}

# This is the custom command 6 for get:

#[test]
def get_extract_the_cpu_list_from_the_sys_information_record_6 [] {
  let result = (sys | get cpu)
  assert ($result == )
}

# This is the custom command 7 for get:

#[test]
def get_getting_pathpath_in_a_case_insensitive_way_7 [] {
  let result = ($env | get paTH)
  assert ($result == )
}

# This is the custom command 8 for get:

#[test]
def get_getting_path_in_a_case_sensitive_way_wont_work_for_path_8 [] {
  let result = ($env | get -s Path)
  assert ($result == )
}


