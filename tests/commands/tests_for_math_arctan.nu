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


# This is the custom command 1 for math_arctan:

#[test]
def math_arctan_get_the_arctangent_of_1_1 [] {
  let result = (1 | math arctan)
  assert ($result == 0.7853981633974483)
}

# This is the custom command 2 for math_arctan:

#[test]
def math_arctan_get_the_arctangent_of__1_in_degrees_2 [] {
  let result = (-1 | math arctan -d)
  assert ($result == -45)
}


