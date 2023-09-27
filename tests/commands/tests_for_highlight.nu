use std assert

# Parameter name:
# sig type   : any
# name       : language
# type       : positional
# shape      : string
# description: language or file extension to help language detection

# Parameter name:
# sig type   : any
# name       : theme
# type       : named
# shape      : string
# description: theme used for highlighting

# Parameter name:
# sig type   : any
# name       : list-themes
# type       : switch
# shape      : 
# description: list all possible themes

# Parameter name:
# sig type   : string
# name       : language
# type       : positional
# shape      : string
# description: language or file extension to help language detection

# Parameter name:
# sig type   : string
# name       : theme
# type       : named
# shape      : string
# description: theme used for highlighting

# Parameter name:
# sig type   : string
# name       : list-themes
# type       : switch
# shape      : 
# description: list all possible themes


# This is the custom command 1 for highlight:

#[test]
def highlight_highlight_a_toml_file_by_its_file_extension_1 [] {
  let result = (open Cargo.toml -r | highlight toml)
  assert ($result == )
}

# This is the custom command 2 for highlight:

#[test]
def highlight_highlight_a_rust_file_by_programming_language_2 [] {
  let result = (open src/main.rs | highlight Rust)
  assert ($result == )
}

# This is the custom command 3 for highlight:

#[test]
def highlight_highlight_a_bash_script_by_inferring_the_language_needs_shebang_3 [] {
  let result = (open example.sh | highlight)
  assert ($result == )
}

# This is the custom command 4 for highlight:

#[test]
def highlight_highlight_a_toml_file_with_another_theme_4 [] {
  let result = (open Cargo.toml -r | highlight toml -t ansi)
  assert ($result == )
}

# This is the custom command 5 for highlight:

#[test]
def highlight_list_all_available_themes_5 [] {
  let result = (highlight --list-themes)
  assert ($result == )
}


