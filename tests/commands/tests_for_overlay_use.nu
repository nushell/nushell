use std assert

# Parameter name:
# sig type   : nothing
# name       : name
# type       : positional
# shape      : string
# description: Module name to use overlay for

# Parameter name:
# sig type   : nothing
# name       : as
# type       : positional
# shape      : "as" string
# description: as keyword followed by a new name

# Parameter name:
# sig type   : nothing
# name       : prefix
# type       : switch
# shape      : 
# description: Prepend module name to the imported commands and aliases

# Parameter name:
# sig type   : nothing
# name       : reload
# type       : switch
# shape      : 
# description: If the overlay already exists, reload its definitions and environment.


# This is the custom command 1 for overlay_use:

#[test]
def overlay_use_create_an_overlay_from_a_module_1 [] {
  let result = (module spam { export def foo [] { "foo" } }
    overlay use spam
    foo)
  assert ($result == )
}

# This is the custom command 2 for overlay_use:

#[test]
def overlay_use_create_an_overlay_from_a_module_and_rename_it_2 [] {
  let result = (module spam { export def foo [] { "foo" } }
    overlay use spam as spam_new
    foo)
  assert ($result == )
}

# This is the custom command 3 for overlay_use:

#[test]
def overlay_use_create_an_overlay_with_a_prefix_3 [] {
  let result = ('export def foo { "foo" }'
    overlay use --prefix spam
    spam foo)
  assert ($result == )
}

# This is the custom command 4 for overlay_use:

#[test]
def overlay_use_create_an_overlay_from_a_file_4 [] {
  let result = ('export-env { $env.FOO = "foo" }' | save spam.nu
    overlay use spam.nu
    $env.FOO)
  assert ($result == )
}


