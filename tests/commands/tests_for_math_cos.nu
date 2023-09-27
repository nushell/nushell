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


# This is the custom command 1 for math_cos:

#[test]
def math_cos_apply_the_cosine_to_π_1 [] {
  let result = (3.141592 | math cos | math round --precision 4)
  assert ($result == -1)
}

# This is the custom command 2 for math_cos:

#[test]
def math_cos_apply_the_cosine_to_a_list_of_angles_in_degrees_2 [] {
  let result = ([0 90 180 270 360] | math cos -d)
  assert ($result == [1, 0, -1, 0, 1])
}


