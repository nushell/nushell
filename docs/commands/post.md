# post
Post content to a URL and retrieve data as a table if possible.

## Usage
```shell
> post <path> <body> {flags} 
 ```

## Parameters
* `<path>` the URL to post to
* `<body>` the contents of the post body

## Flags
* -h, --help: Display this help message
* -u, --user <any>: the username when authenticating
* -p, --password <any>: the password when authenticating
* -t, --content-type <any>: the MIME type of content to post
* -l, --content-length <any>: the length of the content being posted
* -r, --raw: return values as a string instead of a table

