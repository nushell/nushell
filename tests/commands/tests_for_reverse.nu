use std assert


# This is the custom command 1 for reverse:

#[test]
def reverse_reverse_a_list_1 [] {
  let result = ([0,1,2,3] | reverse)
  assert ($result == [3, 2, 1, 0])
}

# This is the custom command 2 for reverse:

#[test]
def reverse_reverse_a_table_2 [] {
  let result = ([{a: 1} {a: 2}] | reverse)
  assert ($result == [{a: 2}, {a: 1}])
}


