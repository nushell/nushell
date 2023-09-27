use std assert

# Parameter name:
# sig type   : nothing
# name       : find
# type       : named
# shape      : string
# description: string to find in alias names and usage


# This is the custom command 1 for help_aliases:

#[test]
def help_aliases_show_all_aliases_1 [] {
  let result = (help aliases)
  assert ($result == )
}

# This is the custom command 2 for help_aliases:

#[test]
def help_aliases_show_help_for_single_alias_2 [] {
  let result = (help aliases my-alias)
  assert ($result == )
}

# This is the custom command 3 for help_aliases:

#[test]
def help_aliases_search_for_string_in_alias_names_and_usages_3 [] {
  let result = (help aliases --find my-alias)
  assert ($result == )
}


