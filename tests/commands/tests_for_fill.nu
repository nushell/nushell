use std assert

# Parameter name:
# sig type   : filesize
# name       : width
# type       : named
# shape      : int
# description: The width of the output. Defaults to 1

# Parameter name:
# sig type   : filesize
# name       : alignment
# type       : named
# shape      : string
# description: The alignment of the output. Defaults to Left (Left(l), Right(r), Center(c/m), MiddleRight(cr/mr))

# Parameter name:
# sig type   : filesize
# name       : character
# type       : named
# shape      : string
# description: The character to fill with. Defaults to ' ' (space)

# Parameter name:
# sig type   : int
# name       : width
# type       : named
# shape      : int
# description: The width of the output. Defaults to 1

# Parameter name:
# sig type   : int
# name       : alignment
# type       : named
# shape      : string
# description: The alignment of the output. Defaults to Left (Left(l), Right(r), Center(c/m), MiddleRight(cr/mr))

# Parameter name:
# sig type   : int
# name       : character
# type       : named
# shape      : string
# description: The character to fill with. Defaults to ' ' (space)

# Parameter name:
# sig type   : list<any>
# name       : width
# type       : named
# shape      : int
# description: The width of the output. Defaults to 1

# Parameter name:
# sig type   : list<any>
# name       : alignment
# type       : named
# shape      : string
# description: The alignment of the output. Defaults to Left (Left(l), Right(r), Center(c/m), MiddleRight(cr/mr))

# Parameter name:
# sig type   : list<any>
# name       : character
# type       : named
# shape      : string
# description: The character to fill with. Defaults to ' ' (space)

# Parameter name:
# sig type   : list<filesize>
# name       : width
# type       : named
# shape      : int
# description: The width of the output. Defaults to 1

# Parameter name:
# sig type   : list<filesize>
# name       : alignment
# type       : named
# shape      : string
# description: The alignment of the output. Defaults to Left (Left(l), Right(r), Center(c/m), MiddleRight(cr/mr))

# Parameter name:
# sig type   : list<filesize>
# name       : character
# type       : named
# shape      : string
# description: The character to fill with. Defaults to ' ' (space)

# Parameter name:
# sig type   : list<int>
# name       : width
# type       : named
# shape      : int
# description: The width of the output. Defaults to 1

# Parameter name:
# sig type   : list<int>
# name       : alignment
# type       : named
# shape      : string
# description: The alignment of the output. Defaults to Left (Left(l), Right(r), Center(c/m), MiddleRight(cr/mr))

# Parameter name:
# sig type   : list<int>
# name       : character
# type       : named
# shape      : string
# description: The character to fill with. Defaults to ' ' (space)

# Parameter name:
# sig type   : list<number>
# name       : width
# type       : named
# shape      : int
# description: The width of the output. Defaults to 1

# Parameter name:
# sig type   : list<number>
# name       : alignment
# type       : named
# shape      : string
# description: The alignment of the output. Defaults to Left (Left(l), Right(r), Center(c/m), MiddleRight(cr/mr))

# Parameter name:
# sig type   : list<number>
# name       : character
# type       : named
# shape      : string
# description: The character to fill with. Defaults to ' ' (space)

# Parameter name:
# sig type   : list<string>
# name       : width
# type       : named
# shape      : int
# description: The width of the output. Defaults to 1

# Parameter name:
# sig type   : list<string>
# name       : alignment
# type       : named
# shape      : string
# description: The alignment of the output. Defaults to Left (Left(l), Right(r), Center(c/m), MiddleRight(cr/mr))

# Parameter name:
# sig type   : list<string>
# name       : character
# type       : named
# shape      : string
# description: The character to fill with. Defaults to ' ' (space)

# Parameter name:
# sig type   : number
# name       : width
# type       : named
# shape      : int
# description: The width of the output. Defaults to 1

# Parameter name:
# sig type   : number
# name       : alignment
# type       : named
# shape      : string
# description: The alignment of the output. Defaults to Left (Left(l), Right(r), Center(c/m), MiddleRight(cr/mr))

# Parameter name:
# sig type   : number
# name       : character
# type       : named
# shape      : string
# description: The character to fill with. Defaults to ' ' (space)

# Parameter name:
# sig type   : string
# name       : width
# type       : named
# shape      : int
# description: The width of the output. Defaults to 1

# Parameter name:
# sig type   : string
# name       : alignment
# type       : named
# shape      : string
# description: The alignment of the output. Defaults to Left (Left(l), Right(r), Center(c/m), MiddleRight(cr/mr))

# Parameter name:
# sig type   : string
# name       : character
# type       : named
# shape      : string
# description: The character to fill with. Defaults to ' ' (space)


# This is the custom command 1 for fill:

#[test]
def fill_fill_a_string_on_the_left_side_to_a_width_of_15_with_the_character__1 [] {
  let result = ('nushell' | fill -a l -c '─' -w 15)
  assert ($result == nushell────────)
}

# This is the custom command 2 for fill:

#[test]
def fill_fill_a_string_on_the_right_side_to_a_width_of_15_with_the_character__2 [] {
  let result = ('nushell' | fill -a r -c '─' -w 15)
  assert ($result == ────────nushell)
}

# This is the custom command 3 for fill:

#[test]
def fill_fill_a_string_on_both_sides_to_a_width_of_15_with_the_character__3 [] {
  let result = ('nushell' | fill -a m -c '─' -w 15)
  assert ($result == ────nushell────)
}

# This is the custom command 4 for fill:

#[test]
def fill_fill_a_number_on_the_left_side_to_a_width_of_5_with_the_character_0_4 [] {
  let result = (1 | fill --alignment right --character '0' --width 5)
  assert ($result == 00001)
}

# This is the custom command 5 for fill:

#[test]
def fill_fill_a_number_on_both_sides_to_a_width_of_5_with_the_character_0_5 [] {
  let result = (1.1 | fill --alignment center --character '0' --width 5)
  assert ($result == 01.10)
}

# This is the custom command 6 for fill:

#[test]
def fill_fill_a_filesize_on_the_left_side_to_a_width_of_5_with_the_character_0_6 [] {
  let result = (1kib | fill --alignment middle --character '0' --width 10)
  assert ($result == 0001024000)
}


