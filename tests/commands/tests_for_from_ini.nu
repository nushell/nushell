use std assert


# This is the custom command 1 for from_ini:

#[test]
def from_ini_converts_ini_formatted_string_to_record_1 [] {
  let result = ('[foo]
a=1
b=2' | from ini)
  assert ($result == {foo: {a: 1, b: 2}})
}


