use std assert


# This is the custom command 1 for from_yaml:

#[test]
def from_yaml_converts_yaml_formatted_string_to_table_1 [] {
  let result = ('a: 1' | from yaml)
  assert ($result == {a: 1})
}

# This is the custom command 2 for from_yaml:

#[test]
def from_yaml_converts_yaml_formatted_string_to_table_2 [] {
  let result = ('[ a: 1, b: [1, 2] ]' | from yaml)
  assert ($result == [{a: 1}, {b: [1, 2]}])
}


