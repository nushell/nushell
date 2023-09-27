use std assert

# Parameter name:
# sig type   : list<number>
# name       : sample
# type       : switch
# shape      : 
# description: calculate sample variance (i.e. using N-1 as the denominator)


# This is the custom command 1 for math_variance:

#[test]
def math_variance_get_the_variance_of_a_list_of_numbers_1 [] {
  let result = ([1 2 3 4 5] | math variance)
  assert ($result == 2)
}

# This is the custom command 2 for math_variance:

#[test]
def math_variance_get_the_sample_variance_of_a_list_of_numbers_2 [] {
  let result = ([1 2 3 4 5] | math variance -s)
  assert ($result == 2.5)
}


