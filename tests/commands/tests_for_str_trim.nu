use std assert

# Parameter name:
# sig type   : list<string>
# name       : char
# type       : named
# shape      : string
# description: character to trim (default: whitespace)

# Parameter name:
# sig type   : list<string>
# name       : left
# type       : switch
# shape      : 
# description: trims characters only from the beginning of the string

# Parameter name:
# sig type   : list<string>
# name       : right
# type       : switch
# shape      : 
# description: trims characters only from the end of the string

# Parameter name:
# sig type   : record
# name       : char
# type       : named
# shape      : string
# description: character to trim (default: whitespace)

# Parameter name:
# sig type   : record
# name       : left
# type       : switch
# shape      : 
# description: trims characters only from the beginning of the string

# Parameter name:
# sig type   : record
# name       : right
# type       : switch
# shape      : 
# description: trims characters only from the end of the string

# Parameter name:
# sig type   : string
# name       : char
# type       : named
# shape      : string
# description: character to trim (default: whitespace)

# Parameter name:
# sig type   : string
# name       : left
# type       : switch
# shape      : 
# description: trims characters only from the beginning of the string

# Parameter name:
# sig type   : string
# name       : right
# type       : switch
# shape      : 
# description: trims characters only from the end of the string

# Parameter name:
# sig type   : table
# name       : char
# type       : named
# shape      : string
# description: character to trim (default: whitespace)

# Parameter name:
# sig type   : table
# name       : left
# type       : switch
# shape      : 
# description: trims characters only from the beginning of the string

# Parameter name:
# sig type   : table
# name       : right
# type       : switch
# shape      : 
# description: trims characters only from the end of the string


# This is the custom command 1 for str_trim:

#[test]
def str_trim_trim_whitespace_1 [] {
  let result = ('Nu shell ' | str trim)
  assert ($result == Nu shell)
}

# This is the custom command 2 for str_trim:

#[test]
def str_trim_trim_a_specific_character_2 [] {
  let result = ('=== Nu shell ===' | str trim -c '=' | str trim)
  assert ($result == Nu shell)
}

# This is the custom command 3 for str_trim:

#[test]
def str_trim_trim_whitespace_from_the_beginning_of_string_3 [] {
  let result = (' Nu shell ' | str trim -l)
  assert ($result == Nu shell )
}

# This is the custom command 4 for str_trim:

#[test]
def str_trim_trim_a_specific_character_4 [] {
  let result = ('=== Nu shell ===' | str trim -c '=')
  assert ($result ==  Nu shell )
}

# This is the custom command 5 for str_trim:

#[test]
def str_trim_trim_whitespace_from_the_end_of_string_5 [] {
  let result = (' Nu shell ' | str trim -r)
  assert ($result ==  Nu shell)
}

# This is the custom command 6 for str_trim:

#[test]
def str_trim_trim_a_specific_character_6 [] {
  let result = ('=== Nu shell ===' | str trim -r -c '=')
  assert ($result == === Nu shell )
}


