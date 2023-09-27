use std assert

# Parameter name:
# sig type   : any
# name       : emoji-name
# type       : positional
# shape      : string
# description: name of the emoji shorthand with colons before and after e.g. :grinning:

# Parameter name:
# sig type   : any
# name       : list
# type       : switch
# shape      : 
# description: List stuff


# This is the custom command 1 for emoji:

#[test]
def emoji_show_the_smirk_emoji_1 [] {
  let result = (emoji :smirk:)
  assert ($result == )
}

# This is the custom command 2 for emoji:

#[test]
def emoji_list_all_known_emojis_2 [] {
  let result = (emoji --list)
  assert ($result == )
}


