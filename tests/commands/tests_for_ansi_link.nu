use std assert

# Parameter name:
# sig type   : list<string>
# name       : cell path
# type       : rest
# shape      : cell-path
# description: for a data structure input, add links to all strings at the given cell paths

# Parameter name:
# sig type   : list<string>
# name       : text
# type       : named
# shape      : string
# description: Link text. Uses uri as text if absent. In case of                 tables, records and lists applies this text to all elements

# Parameter name:
# sig type   : record
# name       : cell path
# type       : rest
# shape      : cell-path
# description: for a data structure input, add links to all strings at the given cell paths

# Parameter name:
# sig type   : record
# name       : text
# type       : named
# shape      : string
# description: Link text. Uses uri as text if absent. In case of                 tables, records and lists applies this text to all elements

# Parameter name:
# sig type   : string
# name       : cell path
# type       : rest
# shape      : cell-path
# description: for a data structure input, add links to all strings at the given cell paths

# Parameter name:
# sig type   : string
# name       : text
# type       : named
# shape      : string
# description: Link text. Uses uri as text if absent. In case of                 tables, records and lists applies this text to all elements

# Parameter name:
# sig type   : table
# name       : cell path
# type       : rest
# shape      : cell-path
# description: for a data structure input, add links to all strings at the given cell paths

# Parameter name:
# sig type   : table
# name       : text
# type       : named
# shape      : string
# description: Link text. Uses uri as text if absent. In case of                 tables, records and lists applies this text to all elements


# This is the custom command 1 for ansi_link:

#[test]
def ansi_link_create_a_link_to_open_some_file_1 [] {
  let result = ('file:///file.txt' | ansi link --text 'Open Me!')
  assert ($result == "\e]8;;file:///file.txt\e\\Open Me!\e]8;;\e\\")
}

# This is the custom command 2 for ansi_link:

#[test]
def ansi_link_create_a_link_without_text_2 [] {
  let result = ('https://www.nushell.sh/' | ansi link)
  assert ($result == "\e]8;;https://www.nushell.sh/\e\\https://www.nushell.sh/\e]8;;\e\\")
}

# This is the custom command 3 for ansi_link:

#[test]
def ansi_link_format_a_table_column_into_links_3 [] {
  let result = ([[url text]; [https://example.com Text]] | ansi link url)
  assert ($result == "")
}


