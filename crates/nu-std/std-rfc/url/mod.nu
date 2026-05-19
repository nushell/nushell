# Quickly edit sections of a url without needing to `url parse` and `url join`

# Helper function to reuse code
def with-field [field: string, value: any]: string -> string {
  url parse
  | update $field $value
  | url join
}

alias 'url with-host' = with-host

# Replace the host of input url
@example 'Change the host portion of an input url to use another search engine' {
    'https://www.google.com/search?q=nushell' | url with-host 'www.bing.com'
} --result 'https://www.bing.com/search?q=nushell'
export def with-host [host: string]: string -> string {
    with-field host $host
}

alias 'url with-port' = with-port

# Set or replace the port of input url
@example 'Define the port for an input url to use' {
    'http://localhost' | url with-port '80'
} --result 'http://localhost:80/'
@example 'Remove the port of an input url' {
    'http://localhost:22/' | url with-port ''
} --result 'http://localhost/'
export def with-port [port: oneof<int, string>]: string -> string {
    with-field port $port
}

alias 'url with-path' = with-path

# Replace the path of input url
@example 'Change the path of a GitHub url to point to another repo' {
    'https://github.com/nushell/nushell/' | url with-path 'nushell/reedline'
} --result 'https://github.com/nushell/reedline'
export def with-path [path: string]: string -> string {
    with-field path $path
}

alias 'url with-fragment' = with-fragment

# Replace the fragment, or anchor, of input url
@example 'Travel to a different section of a Nushell Book chapter' {
    'https://www.nushell.sh/book/types_of_data.html#basic-data-types' | url with-fragment 'types-at-a-glance'
}
export def with-fragment [fragment: string]: string -> string {
    with-field fragment $fragment
}

alias 'url with-params' = with-params

# Set or replace query parameters of input url
#
# Note that they can be provided as a record of key value pairs, or in table form with key and value columns.
@example 'Set parameter values of an input url with a record' {
    let params = {discussions_q: 'is:open is:unanswered'}
    'https://github.com/nushell/nushell/discussions' | url with-params $params
} --result 'https://github.com/nushell/nushell/discussions?discussions_q=is%3Aopen+is%3Aunanswered'
@example 'Set parameter values of an input url with a table' {
    let params = [[key, value]; [q, "is:issue state:open"]]
    'https://github.com/nushell/nushell/issues' | url with-params $params
} --result 'https://github.com/nushell/nushell/issues?q=is%3Aissue+state%3Aopen'
@example 'Remove the query string from an input url by providing an empty value' {
    'https://github.com/nushell/nushell/discussions?discussions_q=is%3Aopen+is%3Aunanswered'
    | url with-params []
} --result 'https://github.com/nushell/nushell/discussions'
export def with-params [query: oneof<record, table<key: string, value: any>>]: string -> string {
    url parse
    | update params $query
    | update query { $query | url build-query }
    | url join
}

alias 'url replace' = replace

# Replace multiple fields of an input url at once using flags
#
# See individual `url with-*` commands' help for more information about valid values.
@example 'Replace a combination of fields of an input url' {
    'https://github.com/nushell/nushell'
    | url replace --host 'www.nushell.sh' --path 'book/types_of_data.html' --fragment '#integers'
} --result 'https://www.nushell.sh/book/types_of_data.html#integers'
export def replace [
	--host: string
	--port: oneof<int, string>
	--path: string
	--fragment: string
	--params: oneof<record, table<key: string, value: any>>
]: string -> string {
    url parse
    | merge (
		{
			host: $host
			port: $port
			path: $path
			fragment: $fragment
			params: $params
			query: (if $params != null { $params | url build-query })
		} | compact
	)
	| url join
}
