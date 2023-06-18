@echo off
echo -------------------------------------------------------------------
echo Building nushell (nu.exe) with dataframes and all the plugins
echo -------------------------------------------------------------------
echo.

echo Building nushell.exe
cargo build --features=dataframe
echo.

call :build crates\nu_plugin_example nu_plugin_example.exe
call :build ..\..\crates\nu_plugin_gstat nu_plugin_gstat.exe
call :build ..\..\crates\nu_plugin_inc nu_plugin_inc.exe
call :build ..\..\crates\nu_plugin_query nu_plugin_query.exe
call :build ..\..\crates\nu_plugin_custom_values nu_plugin_custom_values.exe

cd ..\..
exit /b 0

:build
    setlocal
    set "location=%~1"
    set "target=%~2"

    cd "%location%"
    echo Building %target%
    cargo build
    echo.
    endlocal
exit /b 0
