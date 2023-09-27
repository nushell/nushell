use std assert

# Parameter name:
# sig type   : nothing
# name       : find
# type       : named
# shape      : string
# description: string to find in module names and usage


# This is the custom command 1 for help_modules:

#[test]
def help_modules_show_all_modules_1 [] {
  let result = (help modules)
  assert ($result == )
}

# This is the custom command 2 for help_modules:

#[test]
def help_modules_show_help_for_single_module_2 [] {
  let result = (help modules my-module)
  assert ($result == )
}

# This is the custom command 3 for help_modules:

#[test]
def help_modules_search_for_string_in_module_names_and_usages_3 [] {
  let result = (help modules --find my-module)
  assert ($result == )
}


