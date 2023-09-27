use std assert

# Parameter name:
# sig type   : any
# name       : string
# type       : positional
# shape      : string
# description: String to compare with

# Parameter name:
# sig type   : any
# name       : normalize
# type       : switch
# shape      : 
# description: Normalize the results between 0 and 1

# Parameter name:
# sig type   : any
# name       : list
# type       : switch
# shape      : 
# description: List all available algorithms

# Parameter name:
# sig type   : any
# name       : algorithm
# type       : named
# shape      : string
# description: Name of the algorithm to compute

# Parameter name:
# sig type   : any
# name       : all
# type       : switch
# shape      : 
# description: Run all algorithms


# This is the custom command 1 for str_similarity:

#[test]
def str_similarity_compare_two_strings_for_similarity_1 [] {
  let result = ('nutshell' | str similarity 'nushell')
  assert ($result == )
}

# This is the custom command 2 for str_similarity:

#[test]
def str_similarity_compare_two_strings_for_similarity_and_normalize_the_output_value_2 [] {
  let result = ('nutshell' | str similarity -n 'nushell')
  assert ($result == )
}

# This is the custom command 3 for str_similarity:

#[test]
def str_similarity_compare_two_strings_for_similarity_using_a_specific_algorithm_3 [] {
  let result = ('nutshell' | str similarity 'nushell' -a levenshtein)
  assert ($result == )
}

# This is the custom command 4 for str_similarity:

#[test]
def str_similarity_list_all_the_included_similarity_algorithms_4 [] {
  let result = (str similarity 'nu' --list)
  assert ($result == )
}

# This is the custom command 5 for str_similarity:

#[test]
def str_similarity_compare_two_strings_for_similarity_with_all_algorithms_5 [] {
  let result = ('nutshell' | str similarity 'nushell' -A)
  assert ($result == )
}

# This is the custom command 6 for str_similarity:

#[test]
def str_similarity_compare_two_strings_for_similarity_with_all_algorithms_and_normalize_the_output_value_6 [] {
  let result = ('nutshell' | str similarity 'nushell' -A -n)
  assert ($result == )
}


