use std assert

# Parameter name:
# sig type   : any
# name       : raw
# type       : switch
# shape      : 
# description: remove all of the whitespace

# Parameter name:
# sig type   : any
# name       : indent
# type       : named
# shape      : number
# description: specify indentation width

# Parameter name:
# sig type   : any
# name       : tabs
# type       : named
# shape      : number
# description: specify indentation tab quantity


# This is the custom command 1 for to_json:

#[test]
def to_json_outputs_a_json_string_with_default_indentation_representing_the_contents_of_this_table_1 [] {
  let result = ([a b c] | to json)
  assert ($result == [
  "a",
  "b",
  "c"
])
}

# This is the custom command 2 for to_json:

#[test]
def to_json_outputs_a_json_string_with_4_space_indentation_representing_the_contents_of_this_table_2 [] {
  let result = ([Joe Bob Sam] | to json -i 4)
  assert ($result == [
    "Joe",
    "Bob",
    "Sam"
])
}

# This is the custom command 3 for to_json:

#[test]
def to_json_outputs_an_unformatted_json_string_representing_the_contents_of_this_table_3 [] {
  let result = ([1 2 3] | to json -r)
  assert ($result == [1,2,3])
}


