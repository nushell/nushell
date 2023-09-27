use std assert

# Parameter name:
# sig type   : nothing
# name       : URL
# type       : positional
# shape      : string
# description: the URL to fetch the options from

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
# name       : insecure
# type       : switch
# shape      : 
# description: allow insecure server connections when using SSL

# Parameter name:
# sig type   : nothing
# name       : allow-errors
# type       : switch
# shape      : 
# description: do not fail if the server returns an error code


# This is the custom command 1 for http_options:

#[test]
def http_options_get_options_from_examplecom_1 [] {
  let result = (http options https://www.example.com)
  assert ($result == )
}

# This is the custom command 2 for http_options:

#[test]
def http_options_get_options_from_examplecom_with_username_and_password_2 [] {
  let result = (http options -u myuser -p mypass https://www.example.com)
  assert ($result == )
}

# This is the custom command 3 for http_options:

#[test]
def http_options_get_options_from_examplecom_with_custom_header_3 [] {
  let result = (http options -H [my-header-key my-header-value] https://www.example.com)
  assert ($result == )
}

# This is the custom command 4 for http_options:

#[test]
def http_options_get_options_from_examplecom_with_custom_headers_4 [] {
  let result = (http options -H [my-header-key-A my-header-value-A my-header-key-B my-header-value-B] https://www.example.com)
  assert ($result == )
}

# This is the custom command 5 for http_options:

#[test]
def http_options_simulate_a_browser_cross_origin_preflight_request_from_wwwexamplecom_to_mediaexamplecom_5 [] {
  let result = (http options https://media.example.com/api/ -H [Origin https://www.example.com Access-Control-Request-Headers "Content-Type, X-Custom-Header" Access-Control-Request-Method GET])
  assert ($result == )
}


