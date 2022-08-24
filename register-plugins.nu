# match command
def match [input, matchers: record] {
    echo $matchers | get $input | do $in
}

# get list of all plugin files from their installed directory
let plugin_location = ((which nu).path.0 | path dirname)

# for each plugin file, print the name and launch another instance of nushell to register it
for plugin in (ls $"($plugin_location)/nu_plugin_*") {
    print $"registering ($plugin.name)"
    match ($plugin.name | path basename) {
        # MacOS/Linux
        nu_plugin_custom_values: { nu -c $'register -e msgpack ($plugin.name)' }
        nu_plugin_example: { nu -c $'register -e msgpack ($plugin.name)' }
        nu_plugin_from_parquet: { nu -c $'register -e json ($plugin.name)' }
        nu_plugin_gstat: { nu -c $'register -e msgpack ($plugin.name)' }
        nu_plugin_inc: { nu -c $'register -e json ($plugin.name)' }
        nu_plugin_query: { nu -c $'register -e json ($plugin.name)' }
        # Windows
        nu_plugin_custom_values.exe: { nu -c $'register -e msgpack ($plugin.name)' }
        nu_plugin_example.exe: { nu -c $'register -e msgpack ($plugin.name)' }
        nu_plugin_from_parquet.exe: { nu -c $'register -e json ($plugin.name)' }
        nu_plugin_gstat.exe: { nu -c $'register -e msgpack ($plugin.name)' }
        nu_plugin_inc.exe: { nu -c $'register -e json ($plugin.name)' }
        nu_plugin_query.exe: { nu -c $'register -e json ($plugin.name)' }
    }
}

print "\nplugins registered, please restart nushell"

# print "\nplugin commands registered"
# version | get installed_plugins | split row ', '
