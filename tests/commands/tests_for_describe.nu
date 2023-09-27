use std assert

# Parameter name:
# sig type   : any
# name       : no-collect
# type       : switch
# shape      : 
# description: do not collect streams of structured data


# This is the custom command 1 for describe:

#[test]
def describe_describe_the_type_of_a_string_1 [] {
  let result = ('hello' | describe)
  assert ($result == string)
}


