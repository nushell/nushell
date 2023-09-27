use std assert

# Parameter name:
# sig type   : list<any>
# name       : group_size
# type       : positional
# shape      : int
# description: the size of each group


# This is the custom command 1 for group:

#[test]
def group_group_the_a_list_by_pairs_1 [] {
  let result = ([1 2 3 4] | group 2)
  assert ($result == [[1, 2], [3, 4]])
}


