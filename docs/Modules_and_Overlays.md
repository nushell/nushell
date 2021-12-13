# Modules and Overlays

Similar to many other programming languages, Nushell also has modules that let you import custom commands into a current scope.
However, since Nushell is also a shell, modules allow you to import environment variables which can be used to conveniently activate/deactivate various environments.

## Basics

A simple module can be defined like this:
```
> module greetings {
     export def hello [name: string] {
         $"hello ($name)!"
     }

     export def hi [where: string] {
         $"hi ($where)!"
     }
}
```
We defined `hello` and `hi` custom commands inside a `greetings` module.
The `export` keyword makes it possible to later import the commands from the module.
The collection of exported symbols from a module is called an **overlay**.
You can say that the module `greetings` exports an overlay which consists of two custom commands "hello" and "hi".

By itself, the module does not do anything.
We can verify its existence by printing all available overlays:
```
> $scope.overlays
╭───┬───────────╮
│ 0 │ greetings │
╰───┴───────────╯
```

To actually use its custom commands, we can call `use`:
```
> use greetings

> greetings hello "world"
hello world!

> greetings hi "there"
hi there!
```
The `hello` and `hi` commands are now available with the `greetings` prefix.

In general, anything after the `use` keyword forms an **import pattern** which controls how the symbols are imported.
The import pattern can be one of the following
* Module name (just `greetings`):
    * Imports all symbols with the module name as a prefix
* Module name + command name (`greetings hello`):
    * Import only the selected command into the current scope
* Module name + list of names (`greetings [ hello, hi ]`):
    * Import only the listed commands into the current scope
* Module name + everything (`greetings *`):
    * Imports all names directly into the current scope

We saw the first one already. Let's try the other ones:
```
> use greetings hello

> hello "world"
hello world!

> hi "there" # fails because we brought only 'hello'
```
```
> use greetings [ hello hi ]

> hello "world"
hello world!

> hi "there"
hi there:
```
```
> use greetings *

> hello "world"
hello world!

> hi "there"
hi there!
```

## File as a Module

Typing the module definition to the command line can be tedious.
You could save the module code into a script and `source` it.
However, there is another way that lets Nushell implicitly treat a source file as a module.
Let's start by saving the body of the module definition into a file:
```
# greetings.nu

export def hello [name: string] {
    $"hello ($name)!"
}

export def hi [where: string] {
    $"hi ($where)!"
}
```

Now, you can use `use` directly on the file:
```
> use greetings.nu

> greetings hello "world"
hello world!

> greetings hi "there"
hi there!
```

Nushell automatically infers the module's name from the base name of the file ("greetings" without the ".nu" extension).
You can use any import patterns as described above with the file name instead of the module name.

## Local Custom Commands

Any custom commands defined in a module without the `export` keyword will work only in the module's scope:
```
# greetings.nu

export def hello [name: string] {
    greetings-helper "hello" "world"
}

export def hi [where: string] {
    greetings-helper "hi" "there"
}

def greetings-helper [greeting: string, subject: string] {
    $"($greeting) ($subject)!"
}
```
Then, in Nushell we import all definitions from the "greetings.nu":
```
> use greetings.nu *

> hello "world"
hello world!

> hi "there"
hi there!

> greetings-helper "foo" "bar"  # fails because 'greetings-helper' is not exported
```

## Environment Variables

So far we used modules just to import custom commands.
It is possible to export environment variables the same way.
The syntax is slightly different than what you might be used to from commands like `let-env` or `load-env`:
```
# greetings.nu

export env MYNAME { "Arthur, King of the Britons" }

export def hello [name: string] {
    $"hello ($name)"
}
```
`use` works the same way as with custom commands:
```
> use greetings.nu

> $nu.env."greetings MYNAME"
Arthur, King of the Britons

> greetings hello $nu.env."greetings MYNAME"
hello Arthur, King of the Britons!
```

You can notice we do not assign the value to `MYNAME` directly.
Instead, we give it a block of code (`{ ...}`) that gets evaluated every time we call `use`.
We can demonstrate this property for example with the `random` command:
```
> module roll { export env ROLL { random dice | into string } }

> use roll ROLL

> $nu.env.ROLL
4

> $nu.env.ROLL
4

> use roll ROLL

> $nu.env.ROLL
6

> $nu.env.ROLL
6
```

## Hiding

Any custom command or environment variable, imported from a module or not, can be "hidden", restoring the previous definition.
We do this with the `hide` command:
```
> def foo [] { "foo" }

> foo
foo

> hide foo

> foo  # error! command not found!
```

The `hide` command also accepts import patterns, just like `use`.
The import pattern is interpreted slightly differently, though.
It can be one of the following:
* Module, custom command, or environment variable name (just `foo` or `greetings`):
    * If the name is a custom command or an environment variable, hides it directly. Otherwise:
    * If the name is a module name, hides all of its overlay prefixed with the module name
* Module name + name (`greetings hello`):
    * Hides only the prefixed command / environment variable
* Module name + list of names (`greetings [ hello, hi ]`):
    * Hides only the prefixed commands / environment variables
* Module name + everything (`greetings *`):
    * Hides the whole module's overlay, without the prefix

Let's show these with examples.
We saw direct hiding of a custom command already.
Let's try environment variables:
```
> let-env FOO = "FOO"

> $nu.env.FOO
FOO

> hide FOO

> $nu.env.FOO  # error! environment variable not found!
```
The first case also applies to commands / environment variables brought from a module (using the "greetings.nu" file defined above):
```
> use greetings.nu *

> $nu.env.MYNAME
Arthur, King of the Britons

> hello "world"
hello world!

> hide MYNAME

> $nu.env.MYNAME  # error! environment variable not found!

> hide hello

> hello "world" # error! command not found!
```
And finally, when the name is the module name (assuming the previous `greetings` module):
```
> use greetings.nu

> $nu.env."greetings MYNAME"
Arthur, King of the Britons

> greetings hello "world"
hello world!

> hide greetings

> $nu.env."greetings MYNAME"  # error! environment variable not found!

> greetings hello "world" # error! command not found!
```

To demonstrate the other cases (again, assuming the same `greetings` module):
```
> use greetings.nu

> hide greetings hello

> $nu.env."greetings MYNAME"
Arthur, King of the Britons

> greetings hello "world" # error! command not found!
```
```
> use greetings.nu

> hide greetings [ hello MYNAME ]

> $nu.env."greetings MYNAME" # error! environment variable not found!

> greetings hello "world" # error! command not found!
```
```
> use greetings.nu

> hide greetings *

> $nu.env."greetings MYNAME" # error! environment variable not found!

> greetings hello "world" # error! command not found!
```

## Examples

You can find an example config setup at https://github.com/nushell/nu_scripts/tree/main/engine-q/example-config.
It creates the `$config` variable using the module system.

## Known Issues

* It might be more appropriate to use `$scope.modules` instead of `$scope.overlays`

## Future Design Ideas

The future paragraphs describe some ideas

### Exporting aliases

We should allow exporting aliases as it is a common tool for creating shell environments alongside environment variables.
We need to decide a proper syntax.

### Recursive modules

We should allow using modules within modules.
That is, allowing to use `use` (and `hide`?) within the `module name { ... }` block or a module file.
This leads to a more generic question of having some standard project layout.

### Renaming imports

To avoid name clashing.
For example: `use dataframe as df`.

### Dynamic names for environment variables

The `load-env` command exists because we needed to define the environment variable name at runtime.
Currently, both `let-env` and `export env` require static environment variable names.
Could we allow them to accept an expression in place of the name?
For example `export env (whoami | str screaming-snake-case).0 { "foo" }` or `let-env (whoami | str screaming-snake-case).0 = "foo"`

### To Source or Not To Source

Currently, there are two ways to define a module in a file:
Write the literal `module name { ... }` into a file, use `source` run the file, then `use` to import from the module.
The second way is to use the `use name.nu` directly, which does not require the `module name { ... }` wrapper.
We can keep it as it is, or push into one of the following directions:

1. Rename `source` to `run` and modify it so that it runs in its own scope. Any modifications would be lost, it would be more like running a custom command. This would make it impossible for a random script to modify your environment since the only way to do that would be with the module files and the `use` command. The disadvantage is that it makes it impossible to have "startup scripts" and places some roadblocks to the user experience.
2. Remove `use` and rely on `source` and `module name { ... }` only. This resembles, e.g., Julia's `include(file.jl)` style and makes it quite intuitive. It is not very "pure" or "secure" as dedicated module files with `use`.

We might explore these as we start creating bigger programs and get a feel how a Nushell project structure could look like (and whether or not we should try to enforce one).

## Unlikely Design Ideas

### Exporting variables

`export var name { ... }` which would export a variable the same way you export environment variable.
This would allow for defining global constants in a module (think `math PI`) but can lead to bugs overwriting existing variables.
Use custom commands instead: `export def PI [] { 3.14159 }`.
