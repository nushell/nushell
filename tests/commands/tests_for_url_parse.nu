use std assert


# This is the custom command 1 for url_parse:

#[test]
def url_parse_parses_a_url_1 [] {
  let result = ('http://user123:pass567@www.example.com:8081/foo/bar?param1=section&p2=&f[name]=vldc#hello' | url parse)
  assert ($result == {scheme: http, username: user123, password: pass567, host: www.example.com, port: 8081, path: /foo/bar, query: param1=section&p2=&f[name]=vldc, fragment: hello, params: {param1: section, p2: , f[name]: vldc}})
}


