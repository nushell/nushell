use std assert

# Parameter name:
# sig type   : nothing
# name       : URL
# type       : positional
# shape      : string
# description: the URL to post to

# Parameter name:
# sig type   : nothing
# name       : data
# type       : positional
# shape      : any
# description: the contents of the post body

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
# name       : content-type
# type       : named
# shape      : any
# description: the MIME type of content to post

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
# description: return values as a string instead of a table

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


# This is the custom command 1 for http_patch:

#[test]
def http_patch_patch_content_to_examplecom_1 [] {
  let result = (http patch https://www.example.com 'body')
  assert ($result == )
}

# This is the custom command 2 for http_patch:

#[test]
def http_patch_patch_content_to_examplecom_with_username_and_password_2 [] {
  let result = (http patch -u myuser -p mypass https://www.example.com 'body')
  assert ($result == )
}

# This is the custom command 3 for http_patch:

#[test]
def http_patch_patch_content_to_examplecom_with_custom_header_3 [] {
  let result = (http patch -H [my-header-key my-header-value] https://www.example.com)
  assert ($result == )
}

# This is the custom command 4 for http_patch:

#[test]
def http_patch_patch_content_to_examplecom_with_json_body_4 [] {
  let result = (http patch -t application/json https://www.example.com { field: value })
  assert ($result == )
}


