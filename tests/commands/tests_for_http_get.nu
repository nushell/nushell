use std assert

# Parameter name:
# sig type   : nothing
# name       : URL
# type       : positional
# shape      : string
# description: the URL to fetch the contents from

# Parameter name:
# sig type   : nothing
# name       : user
# type       : named
# shape      : any
# description: the username when authenticating

# Parameter name:
# sig type   : nothing
# name       : password
# type       : named
# shape      : any
# description: the password when authenticating

# Parameter name:
# sig type   : nothing
# name       : max-time
# type       : named
# shape      : int
# description: timeout period in seconds

# Parameter name:
# sig type   : nothing
# name       : headers
# type       : named
# shape      : any
# description: custom headers you want to add 

# Parameter name:
# sig type   : nothing
# name       : raw
# type       : switch
# shape      : 
# description: fetch contents as text rather than a table

# Parameter name:
# sig type   : nothing
# name       : insecure
# type       : switch
# shape      : 
# description: allow insecure server connections when using SSL

# Parameter name:
# sig type   : nothing
# name       : full
# type       : switch
# shape      : 
# description: returns the full response instead of only the body

# Parameter name:
# sig type   : nothing
# name       : allow-errors
# type       : switch
# shape      : 
# description: do not fail if the server returns an error code


# This is the custom command 1 for http_get:

#[test]
def http_get_get_content_from_examplecom_1 [] {
  let result = (http get https://www.example.com)
  assert ($result == )
}

# This is the custom command 2 for http_get:

#[test]
def http_get_get_content_from_examplecom_with_username_and_password_2 [] {
  let result = (http get -u myuser -p mypass https://www.example.com)
  assert ($result == )
}

# This is the custom command 3 for http_get:

#[test]
def http_get_get_content_from_examplecom_with_custom_header_3 [] {
  let result = (http get -H [my-header-key my-header-value] https://www.example.com)
  assert ($result == )
}

# This is the custom command 4 for http_get:

#[test]
def http_get_get_content_from_examplecom_with_custom_headers_4 [] {
  let result = (http get -H [my-header-key-A my-header-value-A my-header-key-B my-header-value-B] https://www.example.com)
  assert ($result == )
}


