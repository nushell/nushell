
# Usage: Just run `powershell install-all.ps1` in nushell root directory

Write-Output "-----------------------------------------------------------------"
Write-Output "Installing nushell (nu) with --features=extra and all the plugins"
Write-Output "-----------------------------------------------------------------"
Write-Output ""

Write-Output "Install nushell from local..."
Write-Output "----------------------------------------------"
cargo install --path . --features=extra

$NU_PLUGINS = @(
    'nu_plugin_example',
    'nu_plugin_gstat',
    'nu_plugin_inc',
    'nu_plugin_query',
    'nu_plugin_custom_values'
)

foreach ( $plugin in $NU_PLUGINS) {
    Write-Output ''
    Write-Output "----------------------------------------------"
    Write-Output "Install plugin $plugin from local..."
    Write-Output "----------------------------------------------"
    Set-Location crates/$plugin
    cargo install --path .
    Set-Location ../../
}

