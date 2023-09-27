use std assert


# This is the custom command 1 for from_nuon:

#[test]
def from_nuon_converts_nuon_formatted_string_to_table_1 [] {
  let result = ('{ a:1 }' | from nuon)
  assert ($result == {a: 1})
}

# This is the custom command 2 for from_nuon:

#[test]
def from_nuon_converts_nuon_formatted_string_to_table_2 [] {
  let result = ('{ a:1, b: [1, 2] }' | from nuon)
  assert ($result == {a: 1, b: [1, 2]})
}


