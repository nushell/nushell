use std assert

# Parameter name:
# sig type   : any
# name       : skip-empty
# type       : switch
# shape      : 
# description: skip empty lines


# This is the custom command 1 for lines:

#[test]
def lines_split_multi_line_string_into_lines_1 [] {
  let result = ($"two\nlines" | lines)
  assert ($result == [two, lines])
}


