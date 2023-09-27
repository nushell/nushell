use std assert

# Parameter name:
# sig type   : table
# name       : right-table
# type       : positional
# shape      : list<any>
# description: The right table in the join

# Parameter name:
# sig type   : table
# name       : left-on
# type       : positional
# shape      : string
# description: Name of column in input (left) table to join on

# Parameter name:
# sig type   : table
# name       : right-on
# type       : positional
# shape      : string
# description: Name of column in right table to join on. Defaults to same column as left table.

# Parameter name:
# sig type   : table
# name       : inner
# type       : switch
# shape      : 
# description: Inner join (default)

# Parameter name:
# sig type   : table
# name       : left
# type       : switch
# shape      : 
# description: Left-outer join

# Parameter name:
# sig type   : table
# name       : right
# type       : switch
# shape      : 
# description: Right-outer join

# Parameter name:
# sig type   : table
# name       : outer
# type       : switch
# shape      : 
# description: Outer join


# This is the custom command 1 for join:

#[test]
def join_join_two_tables_1 [] {
  let result = ([{a: 1 b: 2}] | join [{a: 1 c: 3}] a)
  assert ($result == [{a: 1, b: 2, c: 3}])
}


