use std assert

# Parameter name:
# sig type   : record
# name       : pattern
# type       : positional
# shape      : string
# description: the pattern to output. e.g.) "{foo}: {bar}"

# Parameter name:
# sig type   : table
# name       : pattern
# type       : positional
# shape      : string
# description: the pattern to output. e.g.) "{foo}: {bar}"


# This is the custom command 1 for format:

#[test]
def format_print_filenames_with_their_sizes_1 [] {
  let result = (ls | format '{name}: {size}')
  assert ($result == )
}

# This is the custom command 2 for format:

#[test]
def format_print_elements_from_some_columns_of_a_table_2 [] {
  let result = ([[col1, col2]; [v1, v2] [v3, v4]] | format '{col2}')
  assert ($result == [v2, v4])
}


