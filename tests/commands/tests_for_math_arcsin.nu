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


# This is the custom command 1 for math_arcsin:

#[test]
def math_arcsin_get_the_arcsine_of_1_1 [] {
  let result = (1 | math arcsin)
  assert ($result == 1.5707963267948966)
}

# This is the custom command 2 for math_arcsin:

#[test]
def math_arcsin_get_the_arcsine_of_1_in_degrees_2 [] {
  let result = (1 | math arcsin -d)
  assert ($result == 90)
}


