# Nushell REPL

This directory contains the main Nushell REPL (read eval print loop) as part of the CLI portion of Nushell, which creates the `nu` binary itself.

Current versions of the `nu` binary will use the Nu argument parsing logic to parse the commandline arguments passed to `nu`, leaving the logic here to be a thin layer around what the core libraries.
