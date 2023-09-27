use std assert

# Parameter name:
# sig type   : any
# name       : pretty
# type       : switch
# shape      : 
# description: Formats the Markdown table to vertically align items

# Parameter name:
# sig type   : any
# name       : per-element
# type       : switch
# shape      : 
# description: treat each row as markdown syntax element


# This is the custom command 1 for to_md:

#[test]
def to_md_outputs_an_md_string_representing_the_contents_of_this_table_1 [] {
  let result = ([[foo bar]; [1 2]] | to md)
  assert ($result == |foo|bar|
|-|-|
|1|2|
)
}

# This is the custom command 2 for to_md:

#[test]
def to_md_optionally_output_a_formatted_markdown_string_2 [] {
  let result = ([[foo bar]; [1 2]] | to md --pretty)
  assert ($result == | foo | bar |
| --- | --- |
| 1   | 2   |
)
}

# This is the custom command 3 for to_md:

#[test]
def to_md_treat_each_row_as_a_markdown_element_3 [] {
  let result = ([{"H1": "Welcome to Nushell" } [[foo bar]; [1 2]]] | to md --per-element --pretty)
  assert ($result == # Welcome to Nushell
| foo | bar |
| --- | --- |
| 1   | 2   |)
}

# This is the custom command 4 for to_md:

#[test]
def to_md_render_a_list_4 [] {
  let result = ([0 1 2] | to md --pretty)
  assert ($result == 0
1
2)
}


