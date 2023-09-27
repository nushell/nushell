use std assert

# Parameter name:
# sig type   : list<any>
# name       : all
# type       : switch
# shape      : 
# description: flatten inner table one level out

# Parameter name:
# sig type   : record
# name       : all
# type       : switch
# shape      : 
# description: flatten inner table one level out


# This is the custom command 1 for flatten:

#[test]
def flatten_flatten_a_table_1 [] {
  let result = ([[N, u, s, h, e, l, l]] | flatten )
  assert ($result == [N, u, s, h, e, l, l])
}

# This is the custom command 2 for flatten:

#[test]
def flatten_flatten_a_table_get_the_first_item_2 [] {
  let result = ([[N, u, s, h, e, l, l]] | flatten | first)
  assert ($result == )
}

# This is the custom command 3 for flatten:

#[test]
def flatten_flatten_a_column_having_a_nested_table_3 [] {
  let result = ([[origin, people]; [Ecuador, ([[name, meal]; ['Andres', 'arepa']])]] | flatten --all | get meal)
  assert ($result == )
}

# This is the custom command 4 for flatten:

#[test]
def flatten_restrict_the_flattening_by_passing_column_names_4 [] {
  let result = ([[origin, crate, versions]; [World, ([[name]; ['nu-cli']]), ['0.21', '0.22']]] | flatten versions --all | last | get versions)
  assert ($result == )
}

# This is the custom command 5 for flatten:

#[test]
def flatten_flatten_inner_table_5 [] {
  let result = ({ a: b, d: [ 1 2 3 4 ],  e: [ 4 3  ] } | flatten d --all)
  assert ($result == [{a: b, d: 1, e: [4, 3]}, {a: b, d: 2, e: [4, 3]}, {a: b, d: 3, e: [4, 3]}, {a: b, d: 4, e: [4, 3]}])
}


