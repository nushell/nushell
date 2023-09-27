use std assert

# Parameter name:
# sig type   : list<string>
# name       : cell path
# type       : rest
# shape      : cell-path
# description: for a data structure input, add a gradient to strings at the given cell paths

# Parameter name:
# sig type   : list<string>
# name       : fgstart
# type       : named
# shape      : string
# description: foreground gradient start color in hex (0x123456)

# Parameter name:
# sig type   : list<string>
# name       : fgend
# type       : named
# shape      : string
# description: foreground gradient end color in hex

# Parameter name:
# sig type   : list<string>
# name       : bgstart
# type       : named
# shape      : string
# description: background gradient start color in hex

# Parameter name:
# sig type   : list<string>
# name       : bgend
# type       : named
# shape      : string
# description: background gradient end color in hex

# Parameter name:
# sig type   : record
# name       : cell path
# type       : rest
# shape      : cell-path
# description: for a data structure input, add a gradient to strings at the given cell paths

# Parameter name:
# sig type   : record
# name       : fgstart
# type       : named
# shape      : string
# description: foreground gradient start color in hex (0x123456)

# Parameter name:
# sig type   : record
# name       : fgend
# type       : named
# shape      : string
# description: foreground gradient end color in hex

# Parameter name:
# sig type   : record
# name       : bgstart
# type       : named
# shape      : string
# description: background gradient start color in hex

# Parameter name:
# sig type   : record
# name       : bgend
# type       : named
# shape      : string
# description: background gradient end color in hex

# Parameter name:
# sig type   : string
# name       : cell path
# type       : rest
# shape      : cell-path
# description: for a data structure input, add a gradient to strings at the given cell paths

# Parameter name:
# sig type   : string
# name       : fgstart
# type       : named
# shape      : string
# description: foreground gradient start color in hex (0x123456)

# Parameter name:
# sig type   : string
# name       : fgend
# type       : named
# shape      : string
# description: foreground gradient end color in hex

# Parameter name:
# sig type   : string
# name       : bgstart
# type       : named
# shape      : string
# description: background gradient start color in hex

# Parameter name:
# sig type   : string
# name       : bgend
# type       : named
# shape      : string
# description: background gradient end color in hex

# Parameter name:
# sig type   : table
# name       : cell path
# type       : rest
# shape      : cell-path
# description: for a data structure input, add a gradient to strings at the given cell paths

# Parameter name:
# sig type   : table
# name       : fgstart
# type       : named
# shape      : string
# description: foreground gradient start color in hex (0x123456)

# Parameter name:
# sig type   : table
# name       : fgend
# type       : named
# shape      : string
# description: foreground gradient end color in hex

# Parameter name:
# sig type   : table
# name       : bgstart
# type       : named
# shape      : string
# description: background gradient start color in hex

# Parameter name:
# sig type   : table
# name       : bgend
# type       : named
# shape      : string
# description: background gradient end color in hex


# This is the custom command 1 for ansi_gradient:

#[test]
def ansi_gradient_draw_text_in_a_gradient_with_foreground_start_and_end_colors_1 [] {
  let result = ('Hello, Nushell! This is a gradient.' | ansi gradient --fgstart '0x40c9ff' --fgend '0xe81cff')
  assert ($result == "")
}

# This is the custom command 2 for ansi_gradient:

#[test]
def ansi_gradient_draw_text_in_a_gradient_with_foreground_start_and_end_colors_and_background_start_and_end_colors_2 [] {
  let result = ('Hello, Nushell! This is a gradient.' | ansi gradient --fgstart '0x40c9ff' --fgend '0xe81cff' --bgstart '0xe81cff' --bgend '0x40c9ff')
  assert ($result == "")
}

# This is the custom command 3 for ansi_gradient:

#[test]
def ansi_gradient_draw_text_in_a_gradient_by_specifying_foreground_start_color___end_color_is_assumed_to_be_black_3 [] {
  let result = ('Hello, Nushell! This is a gradient.' | ansi gradient --fgstart '0x40c9ff')
  assert ($result == "")
}

# This is the custom command 4 for ansi_gradient:

#[test]
def ansi_gradient_draw_text_in_a_gradient_by_specifying_foreground_end_color___start_color_is_assumed_to_be_black_4 [] {
  let result = ('Hello, Nushell! This is a gradient.' | ansi gradient --fgend '0xe81cff')
  assert ($result == "")
}


