use std assert

# Parameter name:
# sig type   : nothing
# name       : name
# type       : positional
# shape      : string
# description: Name of the overlay


# This is the custom command 1 for overlay_new:

#[test]
def overlay_new_create_an_empty_overlay_1 [] {
  let result = (overlay new spam)
  assert ($result == )
}


