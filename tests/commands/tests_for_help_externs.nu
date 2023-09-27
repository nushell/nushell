use std assert

# Parameter name:
# sig type   : nothing
# name       : find
# type       : named
# shape      : string
# description: string to find in extern names and usage


# This is the custom command 1 for help_externs:

#[test]
def help_externs_show_all_externs_1 [] {
  let result = (help externs)
  assert ($result == )
}

# This is the custom command 2 for help_externs:

#[test]
def help_externs_show_help_for_single_extern_2 [] {
  let result = (help externs smth)
  assert ($result == )
}

# This is the custom command 3 for help_externs:

#[test]
def help_externs_search_for_string_in_extern_names_and_usages_3 [] {
  let result = (help externs --find smth)
  assert ($result == )
}


