use std assert

# Parameter name:
# sig type   : nothing
# name       : code
# type       : positional
# shape      : any
# description: the name of the code to use like 'green' or 'reset' to reset the color

# Parameter name:
# sig type   : nothing
# name       : escape
# type       : switch
# shape      : 
# description: escape sequence without the escape character(s) ('\x1b[' is not required)

# Parameter name:
# sig type   : nothing
# name       : osc
# type       : switch
# shape      : 
# description: operating system command (osc) escape sequence without the escape character(s) ('\x1b]' is not required)

# Parameter name:
# sig type   : nothing
# name       : list
# type       : switch
# shape      : 
# description: list available ansi code names


# This is the custom command 1 for ansi:

#[test]
def ansi_change_color_to_green_see_how_the_next_example_text_will_be_green_1 [] {
  let result = (ansi green)
  assert ($result == "\e[32m")
}

# This is the custom command 2 for ansi:

#[test]
def ansi_reset_the_color_2 [] {
  let result = (ansi reset)
  assert ($result == "\e[0m")
}

# This is the custom command 3 for ansi:

#[test]
def ansi_use_different_colors_and_styles_in_the_same_text_3 [] {
  let result = ($'(ansi red_bold)Hello(ansi reset) (ansi green_dimmed)Nu(ansi reset) (ansi purple_italic)World(ansi reset)')
  assert ($result == "\e[1;31mHello\e[0m \e[2;32mNu\e[0m \e[3;35mWorld\e[0m")
}

# This is the custom command 4 for ansi:

#[test]
def ansi_the_same_example_as_above_with_short_names_4 [] {
  let result = ($'(ansi rb)Hello(ansi reset) (ansi gd)Nu(ansi reset) (ansi pi)World(ansi reset)')
  assert ($result == "\e[1;31mHello\e[0m \e[2;32mNu\e[0m \e[3;35mWorld\e[0m")
}

# This is the custom command 5 for ansi:

#[test]
def ansi_use_escape_codes_without_the_x1b_5 [] {
  let result = ($"(ansi -e '3;93;41m')Hello(ansi reset)"  # italic bright yellow on red background)
  assert ($result == "\e[3;93;41mHello\e[0m")
}

# This is the custom command 6 for ansi:

#[test]
def ansi_use_structured_escape_codes_6 [] {
  let result = (let bold_blue_on_red = {  # `fg`, `bg`, `attr` are the acceptable keys, all other keys are considered invalid and will throw errors.
        fg: '#0000ff'
        bg: '#ff0000'
        attr: b
    }
    $"(ansi -e $bold_blue_on_red)Hello Nu World(ansi reset)")
  assert ($result == "\e[1;48;2;255;0;0;38;2;0;0;255mHello Nu World\e[0m")
}


