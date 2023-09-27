use std assert

# Parameter name:
# sig type   : list<number>
# name       : sample
# type       : switch
# shape      : 
# description: calculate sample standard deviation (i.e. using N-1 as the denominator)


# This is the custom command 1 for math_stddev:

#[test]
def math_stddev_compute_the_standard_deviation_of_a_list_of_numbers_1 [] {
  let result = ([1 2 3 4 5] | math stddev)
  assert ($result == 1.4142135623730951)
}

# This is the custom command 2 for math_stddev:

#[test]
def math_stddev_compute_the_sample_standard_deviation_of_a_list_of_numbers_2 [] {
  let result = ([1 2 3 4 5] | math stddev -s)
  assert ($result == 1.5811388300841898)
}


