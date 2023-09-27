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
# name       : insecure
# type       : switch
# shape      : 
# description: allow insecure server connections when using SSL


# This is the custom command 1 for http_head:

#[test]
def http_head_get_headers_from_examplecom_1 [] {
  let result = (http head https://www.example.com)
  assert ($result == )
}

# This is the custom command 2 for http_head:

#[test]
def http_head_get_headers_from_examplecom_with_username_and_password_2 [] {
  let result = (http head -u myuser -p mypass https://www.example.com)
  assert ($result == )
}

# This is the custom command 3 for http_head:

#[test]
def http_head_get_headers_from_examplecom_with_custom_header_3 [] {
  let result = (http head -H [my-header-key my-header-value] https://www.example.com)
  assert ($result == )
}


