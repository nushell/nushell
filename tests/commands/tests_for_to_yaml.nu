use std assert


# This is the custom command 1 for to_yaml:

#[test]
def to_yaml_outputs_an_yaml_string_representing_the_contents_of_this_table_1 [] {
  let result = ([[foo bar]; ["1" "2"]] | to yaml)
  assert ($result == - foo: '1'
  bar: '2'
)
}


