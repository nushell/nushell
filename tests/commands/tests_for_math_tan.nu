use std assert

# Parameter name:
# sig type   : list<number>
# name       : degrees
# type       : switch
# shape      : 
# description: Use degrees instead of radians

# Parameter name:
# sig type   : number
# name       : degrees
# type       : switch
# shape      : 
# description: Use degrees instead of radians


# This is the custom command 1 for math_tan:

#[test]
def math_tan_apply_the_tangent_to_π4_1 [] {
  let result = (3.141592 / 4 | math tan | math round --precision 4)
  assert ($result == 1)
}

# This is the custom command 2 for math_tan:

#[test]
def math_tan_apply_the_tangent_to_a_list_of_angles_in_degrees_2 [] {
  let result = ([-45 0 45] | math tan -d)
  assert ($result == [-1, 0, 1])
}


