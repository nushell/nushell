# are we on windows or not?
def windows? [] {
    $nu.os-info.name == windows
}

# filter out files that end in .d
def keep-plugin-executables [] {
    if (windows?) { where name ends-with '.exe' } else { where name !~ '\.d' }
}

# get list of all plugin files from their installed directory
let plugins = (ls ((which nu).path.0 | path dirname) | where name =~ nu_plugin | keep-plugin-executables)
for plugin in $plugins {
    print -n $"registering ($plugin.name), "
    nu -c $"register '($plugin.name)'"
    print "success!"
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

