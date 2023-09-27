use std assert

# Parameter name:
# sig type   : list<any>
# name       : window_size
# type       : positional
# shape      : int
# description: the size of each window

# Parameter name:
# sig type   : list<any>
# name       : stride
# type       : named
# shape      : int
# description: the number of rows to slide over between windows

# Parameter name:
# sig type   : list<any>
# name       : remainder
# type       : switch
# shape      : 
# description: yield last chunks even if they have fewer elements than size


# This is the custom command 1 for window:

#[test]
def window_a_sliding_window_of_two_elements_1 [] {
  let result = ([1 2 3 4] | window 2)
  assert ($result == [[1, 2], [2, 3], [3, 4]])
}

# This is the custom command 2 for window:

#[test]
def window_a_sliding_window_of_two_elements_with_a_stride_of_3_2 [] {
  let result = ([1, 2, 3, 4, 5, 6, 7, 8] | window 2 --stride 3)
  assert ($result == [[1, 2], [4, 5], [7, 8]])
}

# This is the custom command 3 for window:

#[test]
def window_a_sliding_window_of_equal_stride_that_includes_remainder_equivalent_to_chunking_3 [] {
  let result = ([1, 2, 3, 4, 5] | window 3 --stride 3 --remainder)
  assert ($result == [[1, 2, 3], [4, 5]])
}


