use std assert

# Parameter name:
# sig type   : list<any>
# name       : field
# type       : positional
# shape      : cell-path
# description: the name of the column to update

# Parameter name:
# sig type   : list<any>
# name       : replacement value
# type       : positional
# shape      : any
# description: the new value to give the cell(s), or a closure to create the value

# Parameter name:
# sig type   : record
# name       : field
# type       : positional
# shape      : cell-path
# description: the name of the column to update

# Parameter name:
# sig type   : record
# name       : replacement value
# type       : positional
# shape      : any
# description: the new value to give the cell(s), or a closure to create the value

# Parameter name:
# sig type   : table
# name       : field
# type       : positional
# shape      : cell-path
# description: the name of the column to update

# Parameter name:
# sig type   : table
# name       : replacement value
# type       : positional
# shape      : any
# description: the new value to give the cell(s), or a closure to create the value


# This is the custom command 1 for update:

#[test]
def update_update_a_column_value_1 [] {
  let result = ({'name': 'nu', 'stars': 5} | update name 'Nushell')
  assert ($result == {name: Nushell, stars: 5})
}

# This is the custom command 2 for update:

#[test]
def update_use_in_closure_form_for_more_involved_updating_logic_2 [] {
  let result = ([[count fruit]; [1 'apple']] | enumerate | update item.count {|e| ($e.item.fruit | str length) + $e.index } | get item)
  assert ($result == [{count: 5, fruit: apple}])
}

# This is the custom command 3 for update:

#[test]
def update_alter_each_value_in_the_authors_column_to_use_a_single_string_instead_of_a_list_3 [] {
  let result = ([[project, authors]; ['nu', ['Andrés', 'JT', 'Yehuda']]] | update authors {|row| $row.authors | str join ','})
  assert ($result == [{project: nu, authors: Andrés,JT,Yehuda}])
}

# This is the custom command 4 for update:

#[test]
def update_you_can_also_use_a_simple_command_to_update_authors_to_a_single_string_4 [] {
  let result = ([[project, authors]; ['nu', ['Andrés', 'JT', 'Yehuda']]] | update authors {|| str join ','})
  assert ($result == [{project: nu, authors: Andrés,JT,Yehuda}])
}


