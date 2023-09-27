use std assert

# Parameter name:
# sig type   : list<string>
# name       : extension
# type       : named
# shape      : string
# description: Manually supply the extension (without the dot)

# Parameter name:
# sig type   : string
# name       : extension
# type       : named
# shape      : string
# description: Manually supply the extension (without the dot)


# This is the custom command 1 for path_parse:

#[test]
def path_parse_parse_a_single_path_1 [] {
  let result = ('C:\Users\viking\spam.txt' | path parse)
  assert ($result == {prefix: C:, parent: C:\Users\viking, stem: spam, extension: txt})
}

# This is the custom command 2 for path_parse:

#[test]
def path_parse_replace_a_complex_extension_2 [] {
  let result = ('C:\Users\viking\spam.tar.gz' | path parse -e tar.gz | upsert extension { 'txt' })
  assert ($result == )
}

# This is the custom command 3 for path_parse:

#[test]
def path_parse_ignore_the_extension_3 [] {
  let result = ('C:\Users\viking.d' | path parse -e '')
  assert ($result == {prefix: C:, parent: C:\Users, stem: viking.d, extension: })
}

# This is the custom command 4 for path_parse:

#[test]
def path_parse_parse_all_paths_in_a_list_4 [] {
  let result = ([ C:\Users\viking.d C:\Users\spam.txt ] | path parse)
  assert ($result == [{prefix: C:, parent: C:\Users, stem: viking, extension: d}, {prefix: C:, parent: C:\Users, stem: spam, extension: txt}])
}


