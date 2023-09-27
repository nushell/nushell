use std assert


# This is the custom command 1 for length:

#[test]
def length_count_the_number_of_items_in_a_list_1 [] {
  let result = ([1 2 3 4 5] | length)
  assert ($result == 5)
}

# This is the custom command 2 for length:

#[test]
def length_count_the_number_of_rows_in_a_table_2 [] {
  let result = ([{a:1 b:2}, {a:2 b:3}] | length)
  assert ($result == 2)
}


