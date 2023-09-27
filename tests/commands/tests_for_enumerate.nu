use std assert


# This is the custom command 1 for enumerate:

#[test]
def enumerate_add_an_index_to_each_element_of_a_list_1 [] {
  let result = ([a, b, c] | enumerate )
  assert ($result == [{index: 0, item: a}, {index: 1, item: b}, {index: 2, item: c}])
}


