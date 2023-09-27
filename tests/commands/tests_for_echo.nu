use std assert


# This is the custom command 1 for echo:

#[test]
def echo_put_a_list_of_numbers_in_the_pipeline_this_is_the_same_as_1_2_3_1 [] {
  let result = (echo 1 2 3)
  assert ($result == [1, 2, 3])
}

# This is the custom command 2 for echo:

#[test]
def echo_returns_the_piped_in_value_by_using_the_special_in_variable_to_obtain_it_2 [] {
  let result = (echo $in)
  assert ($result == )
}


