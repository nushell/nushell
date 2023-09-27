use std assert

# Parameter name:
# sig type   : nothing
# name       : character
# type       : positional
# shape      : any
# description: the name of the character to output

# Parameter name:
# sig type   : nothing
# name       : list
# type       : switch
# shape      : 
# description: List all supported character names

# Parameter name:
# sig type   : nothing
# name       : unicode
# type       : switch
# shape      : 
# description: Unicode string i.e. 1f378

# Parameter name:
# sig type   : nothing
# name       : integer
# type       : switch
# shape      : 
# description: Create a codepoint from an integer


# This is the custom command 1 for char:

#[test]
def char_output_newline_1 [] {
  let result = (char newline)
  assert ($result == 
)
}

# This is the custom command 2 for char:

#[test]
def char_list_available_characters_2 [] {
  let result = (char --list)
  assert ($result == )
}

# This is the custom command 3 for char:

#[test]
def char_output_prompt_character_newline_and_a_hamburger_menu_character_3 [] {
  let result = ((char prompt) + (char newline) + (char hamburger))
  assert ($result == ▶
≡)
}

# This is the custom command 4 for char:

#[test]
def char_output_unicode_character_4 [] {
  let result = (char -u 1f378)
  assert ($result == 🍸)
}

# This is the custom command 5 for char:

#[test]
def char_create_unicode_from_integer_codepoint_values_5 [] {
  let result = (char -i (0x60 + 1) (0x60 + 2))
  assert ($result == ab)
}

# This is the custom command 6 for char:

#[test]
def char_output_multi_byte_unicode_character_6 [] {
  let result = (char -u 1F468 200D 1F466 200D 1F466)
  assert ($result == 👨‍👦‍👦)
}


