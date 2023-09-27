use std assert

# Parameter name:
# sig type   : nothing
# name       : path
# type       : positional
# shape      : string
# description: path to open

# Parameter name:
# sig type   : string
# name       : path
# type       : positional
# shape      : string
# description: path to open


# This is the custom command 1 for start:

#[test]
def start_open_a_text_file_with_the_default_text_editor_1 [] {
  let result = (start file.txt)
  assert ($result == )
}

# This is the custom command 2 for start:

#[test]
def start_open_an_image_with_the_default_image_viewer_2 [] {
  let result = (start file.jpg)
  assert ($result == )
}

# This is the custom command 3 for start:

#[test]
def start_open_the_current_directory_with_the_default_file_manager_3 [] {
  let result = (start .)
  assert ($result == )
}

# This is the custom command 4 for start:

#[test]
def start_open_a_pdf_with_the_default_pdf_viewer_4 [] {
  let result = (start file.pdf)
  assert ($result == )
}

# This is the custom command 5 for start:

#[test]
def start_open_a_website_with_default_browser_5 [] {
  let result = (start https://www.nushell.sh)
  assert ($result == )
}


