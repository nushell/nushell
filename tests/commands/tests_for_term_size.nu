use std assert


# This is the custom command 1 for term_size:

#[test]
def term_size_return_the_columns_width_and_rows_height_of_the_terminal_1 [] {
  let result = (term size)
  assert ($result == )
}

# This is the custom command 2 for term_size:

#[test]
def term_size_return_the_columns_width_of_the_terminal_2 [] {
  let result = ((term size).columns)
  assert ($result == )
}

# This is the custom command 3 for term_size:

#[test]
def term_size_return_the_rows_height_of_the_terminal_3 [] {
  let result = ((term size).rows)
  assert ($result == )
}


