# Nushell configuration files

## `default_env.nu`:

* The internal default environment variables (other than `$env.config`) that will be set during Nushell startup.
* Is loaded *before* the user's `env.nu`.
* Will be loaded during any startup where the user's `env.nu` is also loaded. For example:
  * During normal startup with `nu`
  * During a startup where the user specifies an alternative `env.nu` via `nu --env-config <path>`
  * During a `nu -c <commandstring>` or `nu <script>` startup so that `ENV_CONVERSIONS` is properly handled for Windows.
* Is *not* loaded when running with an explicit `no --no-config-file (-n)`.
* Is not commented - Comments are in `sample_env.nu`.
* Should be optimized for fastest load times.
* Can be introspected via `config env --default | nu-highlight`

## `default_config.nu`:

Counterpart to `default_env.nu`.

* Contains any `$env.config` values that are not set via Rust defaults.
* Is loaded *after* the user's `env.nu`.
* Is loaded *before* the user's `config.nu`.
* Will be loaded during any startup where the user's `config.nu` is also loaded. For example:
  * During normal startup with `nu`
  * During a startup where the user specifies an alternative `config.nu` via `nu --config <path>`
* Likewise, is never loaded during a startup where the user's `config.nu` would not be loaded. For example:
  * `nu -n/--no-config`
  * `nu -c "ls"`
  * `nu <script.nu>`
* Is not commented - Comments are in `sample_config.nu`.
* Should be optimized for fastest load times. Whenever possible, values should be set via nu-protocol::config
  * Exception: `color_config` values are currently set in this file so that user's can introspect the values
  * TODO: Implement defaults for `color_config` in nu-protocol::config and remove from `default_config.nu`
* Can be introspected via `config nu --default | nu-highlight`
* An ideal `default_config.nu` (when all values are set via `nu-protocol::config`) will simply be:
  ```
  $env.config = {}
  ```

## `sample_env.nu`

* A commented file documenting the most common environment variables that a user might configure in `env.nu`
* For convenient in-shell access - Can be pretty-printed via `config env --sample | nu-highlight`
* Since this file is for documentation only, include actual Nushell code without comments so that it can be pretty-printed
* No optimization necessary - Not intended for use other than documentation.
* Consider replacing `config env --sample` with `help env.nu` at some point.
* Uses a mix of default values (explained) as well as other examples that users might want in their own `env.nu`

## `sample_config.nu`

Counterpart to `sample_env.nu`.

* A commented file documenting the most common environment variables that a user might configure in `config.nu`
* For convenient in-shell access - Can be pretty-printed via `config nu --sample | nu-highlight`
* Since this file is for documentation only, include actual Nushell code without comments so that it can be pretty-printed
* No optimization necessary - Not intended for use other than documentation.
* Consider replacing `config nu --sample` with `help config.nu` at some point.
* Uses a mix of default values (explained) as well as other examples that users might want in their own `config.nu`

## `scaffold_env.nu`

* This file is used *one-time* (typically) at **first** startup
* If the `$nu.default-config-path` directory does not exist, the directory is created and then both `scaffold_env.nu` and `scaffold_config.nu` are written to it
* Contains only commented lines explaining the purpose of the file to the user, along with information on the `config env` command.

## `scaffold_config.nu`

Counterpart to `scaffold_env.nu`.

* This file is used *one-time* (typically) at **first** startup
* If the `$nu.default-config-path` directory does not exist, the directory is created and then both `scaffold_env.nu` and `scaffold_config.nu` are written to it
* Contains only commented lines explaining the purpose of the file to the user, along with information on the `config nu` command.

## `sample_login.nu`

This file is not used by any Nushell code. Of course, if the user has a `login.nu`, then it will be evaluated during startup of a login shell.