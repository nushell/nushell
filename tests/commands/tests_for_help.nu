use std assert

# Parameter name:
# sig type   : nothing
# name       : find
# type       : named
# shape      : string
# description: string to find in command names, usage, and search terms


# This is the custom command 1 for help:

#[test]
def help_show_help_for_single_command_alias_or_module_1 [] {
  let result = (help match)
  assert ($result == )
}

# This is the custom command 2 for help:

#[test]
def help_show_help_for_single_sub_command_alias_or_module_2 [] {
  let result = (help str lpad)
  assert ($result == )
}

# This is the custom command 3 for help:

#[test]
def help_search_for_string_in_command_names_usage_and_search_terms_3 [] {
  let result = (help --find char)
  assert ($result == )
}


