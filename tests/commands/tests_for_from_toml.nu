use std assert


# This is the custom command 1 for from_toml:

#[test]
def from_toml_converts_toml_formatted_string_to_record_1 [] {
  let result = ('a = 1' | from toml)
  assert ($result == {a: 1})
}

# This is the custom command 2 for from_toml:

#[test]
def from_toml_converts_toml_formatted_string_to_record_2 [] {
  let result = ('a = 1
b = [1, 2]' | from toml)
  assert ($result == {a: 1, b: [1, 2]})
}


