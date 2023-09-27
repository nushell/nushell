use std assert

# Parameter name:
# sig type   : any
# name       : html-color
# type       : switch
# shape      : 
# description: change ansi colors to html colors

# Parameter name:
# sig type   : any
# name       : no-color
# type       : switch
# shape      : 
# description: remove all ansi colors in output

# Parameter name:
# sig type   : any
# name       : dark
# type       : switch
# shape      : 
# description: indicate your background color is a darker color

# Parameter name:
# sig type   : any
# name       : partial
# type       : switch
# shape      : 
# description: only output the html for the content itself

# Parameter name:
# sig type   : any
# name       : theme
# type       : named
# shape      : string
# description: the name of the theme to use (github, blulocolight, ...)

# Parameter name:
# sig type   : any
# name       : list
# type       : switch
# shape      : 
# description: produce a color table of all available themes


# This is the custom command 1 for to_html:

#[test]
def to_html_outputs_an__html_string_representing_the_contents_of_this_table_1 [] {
  let result = ([[foo bar]; [1 2]] | to html)
  assert ($result == <html><style>body { background-color:white;color:black; }</style><body><table><thead><tr><th>foo</th><th>bar</th></tr></thead><tbody><tr><td>1</td><td>2</td></tr></tbody></table></body></html>)
}

# This is the custom command 2 for to_html:

#[test]
def to_html_optionally_only_output_the_html_for_the_content_itself_2 [] {
  let result = ([[foo bar]; [1 2]] | to html --partial)
  assert ($result == <div style="background-color:white;color:black;"><table><thead><tr><th>foo</th><th>bar</th></tr></thead><tbody><tr><td>1</td><td>2</td></tr></tbody></table></div>)
}

# This is the custom command 3 for to_html:

#[test]
def to_html_optionally_output_the_string_with_a_dark_background_3 [] {
  let result = ([[foo bar]; [1 2]] | to html --dark)
  assert ($result == <html><style>body { background-color:black;color:white; }</style><body><table><thead><tr><th>foo</th><th>bar</th></tr></thead><tbody><tr><td>1</td><td>2</td></tr></tbody></table></body></html>)
}


