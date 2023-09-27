use std assert

# Parameter name:
# sig type   : string
# name       : objects
# type       : switch
# shape      : 
# description: treat each line as a separate value


# This is the custom command 1 for from_json:

#[test]
def from_json_converts_json_formatted_string_to_table_1 [] {
  let result = ('{ "a": 1 }' | from json)
  assert ($result == {a: 1})
}

# This is the custom command 2 for from_json:

#[test]
def from_json_converts_json_formatted_string_to_table_2 [] {
  let result = ('{ "a": 1, "b": [1, 2] }' | from json)
  assert ($result == {a: 1, b: [1, 2]})
}


