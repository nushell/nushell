@echo off
@echo -------------------------------------------------------------------
@echo Building nushell (nu.exe) with --features=extra and all the plugins
@echo -------------------------------------------------------------------
@echo.

echo Building nushell.exe
cargo build --features=extra
@echo.

@cd crates\nu_plugin_example
echo Building nu_plugin_example.exe
cargo build
@echo.

@cd ..\..\crates\nu_plugin_gstat
echo Building nu_plugin_gstat.exe
cargo build
@echo.

@cd ..\..\crates\nu_plugin_inc
echo Building nu_plugin_inc.exe
cargo build
@echo.

@cd ..\..\crates\nu_plugin_query
echo Building nu_plugin_query.exe
cargo build
@echo.

@cd ..\..\crates\nu_plugin_custom_values
echo Building nu_plugin_custom_values.exe
cargo build
@echo.

@cd ..\..