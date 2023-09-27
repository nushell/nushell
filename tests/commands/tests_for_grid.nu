use std assert

# Parameter name:
# sig type   : list<any>
# name       : width
# type       : named
# shape      : int
# description: number of terminal columns wide (not output columns)

# Parameter name:
# sig type   : list<any>
# name       : color
# type       : switch
# shape      : 
# description: draw output with color

# Parameter name:
# sig type   : list<any>
# name       : separator
# type       : named
# shape      : string
# description: character to separate grid with

# Parameter name:
# sig type   : record
# name       : width
# type       : named
# shape      : int
# description: number of terminal columns wide (not output columns)

# Parameter name:
# sig type   : record
# name       : color
# type       : switch
# shape      : 
# description: draw output with color

# Parameter name:
# sig type   : record
# name       : separator
# type       : named
# shape      : string
# description: character to separate grid with

# Parameter name:
# sig type   : table
# name       : width
# type       : named
# shape      : int
# description: number of terminal columns wide (not output columns)

# Parameter name:
# sig type   : table
# name       : color
# type       : switch
# shape      : 
# description: draw output with color

# Parameter name:
# sig type   : table
# name       : separator
# type       : named
# shape      : string
# description: character to separate grid with


# This is the custom command 1 for grid:

#[test]
def grid_render_a_simple_list_to_a_grid_1 [] {
  let result = ([1 2 3 a b c] | grid)
  assert ($result == 1 │ 2 │ 3 │ a │ b │ c
)
}

# This is the custom command 2 for grid:

#[test]
def grid_the_above_example_is_the_same_as_2 [] {
  let result = ([1 2 3 a b c] | wrap name | grid)
  assert ($result == 1 │ 2 │ 3 │ a │ b │ c
)
}

# This is the custom command 3 for grid:

#[test]
def grid_render_a_record_to_a_grid_3 [] {
  let result = ({name: 'foo', b: 1, c: 2} | grid)
  assert ($result == foo
)
}

# This is the custom command 4 for grid:

#[test]
def grid_render_a_list_of_records_to_a_grid_4 [] {
  let result = ([{name: 'A', v: 1} {name: 'B', v: 2} {name: 'C', v: 3}] | grid)
  assert ($result == A │ B │ C
)
}

# This is the custom command 5 for grid:

#[test]
def grid_render_a_table_with_name_column_in_it_to_a_grid_5 [] {
  let result = ([[name patch]; [0.1.0 false] [0.1.1 true] [0.2.0 false]] | grid)
  assert ($result == 0.1.0 │ 0.1.1 │ 0.2.0
)
}


