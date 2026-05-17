# Nushell core libraries and plugins

These sub-crates form both the foundation for Nu and a set of plugins which extend Nu with additional functionality.

Foundational libraries are split into two kinds of crates:

* Core crates - those crates that work together to build the Nushell language engine
* Support crates - a set of crates that support the engine with additional features like JSON support, ANSI support, and more.

Plugins are likewise also split into two types:

* Core plugins - plugins that provide part of the default experience of Nu, including access to the system properties, processes, and web-connectivity features.
* Extra plugins - these plugins run a wide range of different capabilities like working with different file types, charting, viewing binary data, and more.
