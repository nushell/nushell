use std assert

# Parameter name:
# sig type   : nothing
# name       : name
# type       : positional
# shape      : string
# description: name of the alias

# Parameter name:
# sig type   : nothing
# name       : initial_value
# type       : positional
# shape      : "=" expression
# description: equals sign followed by value


# This is the custom command 1 for export_alias:

#[test]
def export_alias_alias_ll_to_ls__l_and_export_it_from_a_module_1 [] {
  let result = (module spam { export alias ll = ls -l })
  assert ($result == )
}


