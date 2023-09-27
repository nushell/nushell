use std assert

# Parameter name:
# sig type   : list<number>
# name       : degrees
# type       : switch
# shape      : 
# description: Return degrees instead of radians

# Parameter name:
# sig type   : number
# name       : degrees
# type       : switch
# shape      : 
# description: Return degrees instead of radians


# This is the custom command 1 for math_arccos:

#[test]
def math_arccos_get_the_arccosine_of_1_1 [] {
  let result = (1 | math arccos)
  assert ($result == 0)
}

# This is the custom command 2 for math_arccos:

#[test]
def math_arccos_get_the_arccosine_of__1_in_degrees_2 [] {
  let result = (-1 | math arccos -d)
  assert ($result == 180)
}


