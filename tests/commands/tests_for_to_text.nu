use std assert


# This is the custom command 1 for to_text:

#[test]
def to_text_outputs_data_as_simple_text_1 [] {
  let result = (1 | to text)
  assert ($result == 1)
}

# This is the custom command 2 for to_text:

#[test]
def to_text_outputs_external_data_as_simple_text_2 [] {
  let result = (git help -a | lines | find -r '^ ' | to text)
  assert ($result == )
}

# This is the custom command 3 for to_text:

#[test]
def to_text_outputs_records_as_simple_text_3 [] {
  let result = (ls | to text)
  assert ($result == )
}


