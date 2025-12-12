The nushell extension gives you run nushell specific commands and other shell commands.
This extension should be preferred over other tools for running shell commands as it can run both nushell comamands and other shell commands.

Native nushell commands return structured content in NUON format (no need to pipe to `| to json`).
Native nushell commands can be discovered by using the list_commands tool.
Prefer nushell native commands where possible as they provide structured data in a pipeline, versus text output.
To discover the input (stdin) and output (stdout) types of a command, flags, and positional arguments use the command_help tool.

Nushell native commands will return structured content. Piping of commands that return a table, list, or record to `to text` will return plain text.
In order to find out what columns are available use the `columns` command. For example `ps | columns` will return the columns available from the `ps` command.

HTTP request examples:
```nu
# GET request
http get https://api.example.com/data

# POST with JSON body
http post --content-type application/json https://api.example.com/endpoint {foo: "bar", baz: 123}

# POST with custom headers and empty body
http post https://api.example.com/sync -H {X-API-Key: "secret"} (bytes build)

# POST with headers and JSON body
http post --content-type application/json https://api.example.com/data -H {Authorization: "Bearer token"} {key: "value"}
```

**String interpolation:** Use `$"...(expr)..."` syntax. Variables/expressions must be in parentheses inside `$"..."` strings.
```nu
# BAD - regular strings don't interpolate variables
let name = "world"; echo "hello $name"   # Prints literal: hello $name

# GOOD - use $"..." with parentheses around expressions
let name = "world"; echo $"hello ($name)"       # Prints: hello world
ls $"($env.HOME)/Documents"                     # Expands path correctly
cargo build $"--jobs=(sys cpu | length)"        # Dynamic flag value
```

**Command-line flags with variables:** When building flags that include variable values, the entire flag must be an interpolated string.
```nu
# BAD - mixing bash-style and nushell syntax
mysql -p"$env.DATABASE_PASSWORD" mydb           # ERROR: $env not expanded
mysql -p $env.DATABASE_PASSWORD mydb            # ERROR: password becomes separate arg

# GOOD - entire flag as interpolated string
mysql $"-p($env.DATABASE_PASSWORD)" mydb        # Password correctly embedded
curl -H $"Authorization: Bearer ($token)"       # Header with variable

# GOOD - alternative: use flag=value syntax if supported
mysql $"--password=($env.DATABASE_PASSWORD)" mydb
```

**SQL queries with wildcards:** The `*` character is interpreted as a glob pattern in nushell. Escape or quote carefully.
```nu
# BAD - asterisk interpreted as glob
let query = "SELECT COUNT(*) FROM users"        # ERROR: Command `*` not found
mysql -e "SELECT * FROM users"                  # ERROR: glob expansion

# GOOD - use single quotes for literal SQL (no interpolation needed)
mysql -e 'SELECT COUNT(*) FROM users'           # Works: single quotes are literal

# GOOD - if you need interpolation, escape or use raw string
let table = "users"
mysql -e $"SELECT COUNT\(*\) FROM ($table)"     # Escaped asterisks
```

To find a nushell command or to see all available commands use the list_commands tool.
To learn more about how to use a command, use the command_help tool.
You can use the eval tool to run any command that would work on the relevant operating system.
Use the eval tool as needed to locate files or interact with the project.
