use std assert

# Parameter name:
# sig type   : any
# name       : raw
# type       : switch
# shape      : 
# description: Prints the raw value representation

# Parameter name:
# sig type   : list<any>
# name       : raw
# type       : switch
# shape      : 
# description: Prints the raw value representation

# Parameter name:
# sig type   : table
# name       : raw
# type       : switch
# shape      : 
# description: Prints the raw value representation


# This is the custom command 1 for debug:

#[test]
def debug_debug_print_a_string_1 [] {
  let result = ('hello' | debug)
  assert ($result == hello)
}

# This is the custom command 2 for debug:

#[test]
def debug_debug_print_a_list_2 [] {
  let result = (['hello'] | debug)
  assert ($result == [hello])
}

# This is the custom command 3 for debug:

#[test]
def debug_debug_print_a_table_3 [] {
  let result = ([[version patch]; ['0.1.0' false] ['0.1.1' true] ['0.2.0' false]] | debug)
  assert ($result == [{version: 0.1.0, patch: false}, {version: 0.1.1, patch: true}, {version: 0.2.0, patch: false}])
}


