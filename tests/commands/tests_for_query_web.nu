use std assert

# Parameter name:
# sig type   : any
# name       : query
# type       : named
# shape      : string
# description: selector query

# Parameter name:
# sig type   : any
# name       : as-html
# type       : switch
# shape      : 
# description: return the query output as html

# Parameter name:
# sig type   : any
# name       : attribute
# type       : named
# shape      : string
# description: downselect based on the given attribute

# Parameter name:
# sig type   : any
# name       : as-table
# type       : named
# shape      : table
# description: find table based on column header list

# Parameter name:
# sig type   : any
# name       : inspect
# type       : switch
# shape      : 
# description: run in inspect mode to provide more information for determining column headers


# This is the custom command 1 for query_web:

#[test]
def query_web_retrieve_all_header_elements_from_phoronixcom_website_1 [] {
  let result = (http get https://phoronix.com | query web -q 'header')
  assert ($result == )
}

# This is the custom command 2 for query_web:

#[test]
def query_web_retrieve_a_html_table_from_wikipedia_and_parse_it_into_a_nushell_table_using_table_headers_as_guides_2 [] {
  let result = (http get https://en.wikipedia.org/wiki/List_of_cities_in_India_by_population
    | query web -t [Rank City 'Population(2011)[3]' 'Population(2001)[3][a]' 'State or union territory'])
  assert ($result == )
}

# This is the custom command 3 for query_web:

#[test]
def query_web_pass_multiple_css_selectors_to_extract_several_elements_within_single_query_group_the_query_results_together_and_rotate_them_to_create_a_table_3 [] {
  let result = (http get https://www.nushell.sh | query web -q 'h2, h2 + p' | group 2 | each {rotate --ccw tagline description} | flatten)
  assert ($result == )
}

# This is the custom command 4 for query_web:

#[test]
def query_web_retrieve_a_specific_html_attribute_instead_of_the_default_text_4 [] {
  let result = (http get https://example.org | query web --query a --attribute href)
  assert ($result == )
}


