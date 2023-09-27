use std assert


# This is the custom command 1 for shuffle:

#[test]
def shuffle_shuffle_rows_randomly_execute_it_several_times_and_see_the_difference_1 [] {
  let result = ([[version patch]; ['1.0.0' false] ['3.0.1' true] ['2.0.0' false]] | shuffle)
  assert ($result == )
}


