use std assert

# Parameter name:
# sig type   : any
# name       : start-number
# type       : named
# shape      : int
# description: row number to start viewing from

# Parameter name:
# sig type   : any
# name       : list
# type       : switch
# shape      : 
# description: list available table modes/themes

# Parameter name:
# sig type   : any
# name       : width
# type       : named
# shape      : int
# description: number of terminal columns wide (not output columns)

# Parameter name:
# sig type   : any
# name       : expand
# type       : switch
# shape      : 
# description: expand the table structure in a light mode

# Parameter name:
# sig type   : any
# name       : expand-deep
# type       : named
# shape      : int
# description: an expand limit of recursion which will take place

# Parameter name:
# sig type   : any
# name       : flatten
# type       : switch
# shape      : 
# description: Flatten simple arrays

# Parameter name:
# sig type   : any
# name       : flatten-separator
# type       : named
# shape      : string
# description: sets a separator when 'flatten' used

# Parameter name:
# sig type   : any
# name       : collapse
# type       : switch
# shape      : 
# description: expand the table structure in collapse mode. Be aware collapse mode currently doesn't support width control

# Parameter name:
# sig type   : any
# name       : abbreviated
# type       : named
# shape      : int
# description: abbreviate the data in the table by truncating the middle part and only showing amount provided on top and bottom


# This is the custom command 1 for table:

#[test]
def table_list_the_files_in_current_directory_with_indexes_starting_from_1_1 [] {
  let result = (ls | table -n 1)
  assert ($result == )
}

# This is the custom command 2 for table:

#[test]
def table_render_data_in_table_view_2 [] {
  let result = ([[a b]; [1 2] [3 4]] | table)
  assert ($result == [{a: 1, b: 2}, {a: 3, b: 4}])
}

# This is the custom command 3 for table:

#[test]
def table_render_data_in_table_view_expanded_3 [] {
  let result = ([[a b]; [1 2] [2 [4 4]]] | table --expand)
  assert ($result == [{a: 1, b: 2}, {a: 3, b: 4}])
}

# This is the custom command 4 for table:

#[test]
def table_render_data_in_table_view_collapsed_4 [] {
  let result = ([[a b]; [1 2] [2 [4 4]]] | table --collapse)
  assert ($result == [{a: 1, b: 2}, {a: 3, b: 4}])
}


