use std assert

# Parameter name:
# sig type   : nothing
# name       : search
# type       : positional
# shape      : string
# description: item to search for, or 'list' to list available tutorials

# Parameter name:
# sig type   : nothing
# name       : find
# type       : named
# shape      : string
# description: Search tutorial for a phrase


# This is the custom command 1 for tutor:

#[test]
def tutor_begin_the_tutorial_1 [] {
  let result = (tutor begin)
  assert ($result == )
}

# This is the custom command 2 for tutor:

#[test]
def tutor_search_a_tutorial_by_phrase_2 [] {
  let result = (tutor -f "$in")
  assert ($result == )
}


