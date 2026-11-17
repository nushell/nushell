# Environment variables: reading, setting, and scoped env.
# Watch the Globals → $env scope in the Variables panel update.

# Reading existing env vars
print $"Current directory: ($env.PWD)"
print $"Home: ($env.HOME? | default 'not set')"

# Setting env vars — shows up in the Environment scope
$env.MY_APP_MODE = "debug"
$env.MY_APP_PORT = "8080"
$env.MY_APP_VERSION = "1.2.3"
print $"App mode: ($env.MY_APP_MODE)"

# Env vars in commands
let config_str = $"($env.MY_APP_MODE):($env.MY_APP_PORT)"
print $"Config: ($config_str)"

# Scoped environment with `with-env`
print "\n--- Scoped env ---"
with-env { DATABASE_URL: "postgres://localhost/test" } {
    print $"Inside: DB=($env.DATABASE_URL)"
}
# DATABASE_URL no longer exists here

# Building PATH-like variables
$env.MY_SEARCH_PATH = ["/usr/local/bin", "/usr/bin", "/bin"]
let full_path = $env.MY_SEARCH_PATH | str join ":"
print $"Search path: ($full_path)"

# Environment and conditionals
$env.DEPLOY_ENV = "staging"
let db = match $env.DEPLOY_ENV {
    "production" => "prod-db.internal"
    "staging" => "staging-db.internal"
    _ => "localhost"
}
print $"Database host: ($db)"
