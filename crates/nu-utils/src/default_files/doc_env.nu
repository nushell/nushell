# Nushell Environment Config File Documentation
#
# version = "0.107.1"
#
# Previously, environment variables were typically configured in `env.nu`.
# In general, most configuration can and should be performed in `config.nu`
# or one of the autoload directories.

# To pretty-print the in-shell documentation for Nushell's various configuration
# settings, you can run:
config nu --doc | nu-highlight | less -R