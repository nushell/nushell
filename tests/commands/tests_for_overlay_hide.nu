use std assert

# Parameter name:
# sig type   : nothing
# name       : name
# type       : positional
# shape      : string
# description: Overlay to hide

# Parameter name:
# sig type   : nothing
# name       : keep-custom
# type       : switch
# shape      : 
# description: Keep all newly added commands and aliases in the next activated overlay

# Parameter name:
# sig type   : nothing
# name       : keep-env
# type       : named
# shape      : list<string>
# description: List of environment variables to keep in the next activated overlay


# This is the custom command 1 for overlay_hide:

#[test]
def overlay_hide_keep_a_custom_command_after_hiding_the_overlay_1 [] {
  let result = (module spam { export def foo [] { "foo" } }
    overlay use spam
    def bar [] { "bar" }
    overlay hide spam --keep-custom
    bar
    )
  assert ($result == )
}

# This is the custom command 2 for overlay_hide:

#[test]
def overlay_hide_hide_an_overlay_created_from_a_file_2 [] {
  let result = ('export alias f = "foo"' | save spam.nu
    overlay use spam.nu
    overlay hide spam)
  assert ($result == )
}

# This is the custom command 3 for overlay_hide:

#[test]
def overlay_hide_hide_the_last_activated_overlay_3 [] {
  let result = (module spam { export-env { $env.FOO = "foo" } }
    overlay use spam
    overlay hide)
  assert ($result == )
}

# This is the custom command 4 for overlay_hide:

#[test]
def overlay_hide_keep_the_current_working_directory_when_removing_an_overlay_4 [] {
  let result = (overlay new spam
    cd some-dir
    overlay hide --keep-env [ PWD ] spam)
  assert ($result == )
}


