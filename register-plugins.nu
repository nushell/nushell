# match command
def match [input, matchers: record] {
    echo $matchers | get $input | do $in
}

# register plugin
def register_plugin [plugin] {
    print -n $"registering ($plugin), "
    nu -c $'register ($plugin)'
    print "success!"
}

# are we on windows or not?
def windows? [] {
    $nu.os-info.name == windows
}

# filter out files that end in .d
def keep-plugin-executables [] {
    if (windows?) { $in } else { where name !~ '\.d' }
}

# get list of all plugin files from their installed directory
let plugin_location = ((which nu).path.0 | path dirname)

# for each plugin file, print the name and launch another instance of nushell to register it
for plugin in (ls $"($plugin_location)/nu_plugin_*" | keep-plugin-executables) {
    match ($plugin.name | path basename | str replace '\.exe$' '') {
        nu_plugin_custom_values: { register_plugin $plugin.name }
        nu_plugin_example: { register_plugin $plugin.name }
        nu_plugin_from_parquet: { register_plugin $plugin.name }
        nu_plugin_gstat: { register_plugin $plugin.name }
        nu_plugin_inc: { register_plugin $plugin.name }
        nu_plugin_query: { register_plugin $plugin.name }
        nu_plugin_regex: { register_plugin $plugin.name }
        nu_plugin_periodic_table: { register_plugin $plugin.name }
        nu_plugin_pnet: { register_plugin $plugin.name }
        nu_plugin_python: { register_plugin $plugin.name }
        nu_plugin_bio: { register_plugin $plugin.name }
        nu_plugin_dcm: { register_plugin $plugin.name }
        nu_plugin_dotenv: { register_plugin $plugin.name }
        nu_plugin_from_bencode: { register_plugin $plugin.name }
    }
}

# print helpful message
print "\nplugins registered, please restart nushell"

# Plugin Location
# https://github.com/nushell/nushell/tree/main/crates/nu_plugin_custom_values
# https://github.com/nushell/nushell/tree/main/crates/nu_plugin_example
# https://github.com/nushell/nushell/tree/main/crates/nu_plugin_gstat
# https://github.com/nushell/nushell/tree/main/crates/nu_plugin_inc
# https://github.com/nushell/nushell/tree/main/crates/nu_plugin_python
# https://github.com/nushell/nushell/tree/main/crates/nu_plugin_query
# https://github.com/fdncred/nu_plugin_from_parquet
# https://github.com/fdncred/nu_plugin_from_regex
# https://github.com/fdncred/nu_plugin_pnet
# https://github.com/JosephTLyons/nu_plugin_periodic_table
# https://github.com/Euphrasiologist/nu_plugin_bio
# https://github.com/realcundo/nu_plugin_dcm
# https://github.com/enerdgumen/nu_plugin_dotenv
# https://github.com/bluk/nu_plugin_from_bencode

# Older plugins
# https://github.com/notryanb/nu_plugin_id3
# https://github.com/notryanb/nu_plugin_weather
# https://github.com/tiffany352/nu-plugins/tree/main/from_nbt
# https://github.com/tiffany352/nu-plugins/tree/main/file_exists
# https://github.com/potan/nu_plugin_wifiscan
# https://github.com/autophagy/nu_plugin_from_dhall
# https://github.com/yanganto/nu_plugin_s3
# https://github.com/lukasreuter/nu_plugin_unity
# https://github.com/filaretov/nu_plugin_path_temp
# https://github.com/cdecompilador/nu_plugin_bg
# https://github.com/aJuvan/nu_plugin_kubectl
# https://github.com/hedonihilist/nu_plugin_df

