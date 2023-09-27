use std assert


# This is the custom command 1 for to_toml:

#[test]
def to_toml_outputs_an_toml_string_representing_the_contents_of_this_record_1 [] {
  let result = ({foo: 1 bar: 'qwe'} | to toml)
  assert ($result == bar = "qwe"
foo = 1
)
}


