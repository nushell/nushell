# Coloring and Theming in Nushell

There are a few main parts that nushell allows you to change the color. All of these can be set in the `config.nu` configuration file. If you see the hash/hashtag/pound mark `#` in the config file it means the text after it is commented out.

1. table borders
2. primitive values
3. flatshapes (this is the command line syntax)
4. prompt
5. LS_COLORS

## `Table borders`
___

Table borders are controlled by the `table-mode` setting in the `config.nu`. Here is an example:
```
let $config = {
    table-mode: rounded
}
```

Here are the current options for `table-mode`:
1. `rounded` # of course, this is the best one :)
2. `basic`
3. `compact`
4. `compact-double`
5. `light`
6. `thin`
7. `with-love`
8. `rounded`
9. `reinforced`
10. `heavy`
11. `none`
12. `other`

### `Color symbologies`
---

* `r` - normal color red's abbreviation
* `rb` - normal color red's abbreviation with bold attribute
* `red` - normal color red
* `red-bold` - normal color red with bold attribute
* `"#ff0000"` - "#hex" format foreground color red (quotes are required)
* `{ fg: "#ff0000" bg: "#0000ff" attr: b }` - "full #hex" format foreground red in "#hex" format with a background of blue in "#hex" format with an attribute of bold abbreviated.

### `attributes`
---

|code|meaning|
|-|-|
|l|blink|
|b|bold|
|d|dimmed|
|h|hidden|
|i|italic|
|r|reverse|
|s|strikethrough|
|u|underline|
|n|nothing|
||defaults to nothing|

### `normal colors` and `abbreviations`

|code|name|
|-|-|
|g|green|
|gb|green-bold|
|gu|green-underline|
|gi|green-italic|
|gd|green-dimmed|
|gr|green-reverse|
|gbl|green-blink|
|gst|green-strike|
|lg|light-green|
|lgb|light-green-bold|
|lgu|light-green-underline|
|lgi|light-green-italic|
|lgd|light-green-dimmed|
|lgr|light-green-reverse|
|lgbl|light-green-blink|
|lgst|light-green-strike|
|r|red|
|rb|red-bold|
|ru|red-underline|
|ri|red-italic|
|rd|red-dimmed|
|rr|red-reverse|
|rbl|red-blink|
|rst|red-strike|
|lr|light-red|
|lrb|light-red-bold|
|lru|light-red-underline|
|lri|light-red-italic|
|lrd|light-red-dimmed|
|lrr|light-red-reverse|
|lrbl|light-red-blink|
|lrst|light-red-strike|
|u|blue|
|ub|blue-bold|
|uu|blue-underline|
|ui|blue-italic|
|ud|blue-dimmed|
|ur|blue-reverse|
|ubl|blue-blink|
|ust|blue-strike|
|lu|light-blue|
|lub|light-blue-bold|
|luu|light-blue-underline|
|lui|light-blue-italic|
|lud|light-blue-dimmed|
|lur|light-blue-reverse|
|lubl|light-blue-blink|
|lust|light-blue-strike|
|b|black|
|bb|black-bold|
|bu|black-underline|
|bi|black-italic|
|bd|black-dimmed|
|br|black-reverse|
|bbl|black-blink|
|bst|black-strike|
|ligr|light-gray|
|ligrb|light-gray-bold|
|ligru|light-gray-underline|
|ligri|light-gray-italic|
|ligrd|light-gray-dimmed|
|ligrr|light-gray-reverse|
|ligrbl|light-gray-blink|
|ligrst|light-gray-strike|
|y|yellow|
|yb|yellow-bold|
|yu|yellow-underline|
|yi|yellow-italic|
|yd|yellow-dimmed|
|yr|yellow-reverse|
|ybl|yellow-blink|
|yst|yellow-strike|
|ly|light-yellow|
|lyb|light-yellow-bold|
|lyu|light-yellow-underline|
|lyi|light-yellow-italic|
|lyd|light-yellow-dimmed|
|lyr|light-yellow-reverse|
|lybl|light-yellow-blink|
|lyst|light-yellow-strike|
|p|purple|
|pb|purple-bold|
|pu|purple-underline|
|pi|purple-italic|
|pd|purple-dimmed|
|pr|purple-reverse|
|pbl|purple-blink|
|pst|purple-strike|
|lp|light-purple|
|lpb|light-purple-bold|
|lpu|light-purple-underline|
|lpi|light-purple-italic|
|lpd|light-purple-dimmed|
|lpr|light-purple-reverse|
|lpbl|light-purple-blink|
|lpst|light-purple-strike|
|c|cyan|
|cb|cyan-bold|
|cu|cyan-underline|
|ci|cyan-italic|
|cd|cyan-dimmed|
|cr|cyan-reverse|
|cbl|cyan-blink|
|cst|cyan-strike|
|lc|light-cyan|
|lcb|light-cyan-bold|
|lcu|light-cyan-underline|
|lci|light-cyan-italic|
|lcd|light-cyan-dimmed|
|lcr|light-cyan-reverse|
|lcbl|light-cyan-blink|
|lcst|light-cyan-strike|
|w|white|
|wb|white-bold|
|wu|white-underline|
|wi|white-italic|
|wd|white-dimmed|
|wr|white-reverse|
|wbl|white-blink|
|wst|white-strike|
|dgr|dark-gray|
|dgrb|dark-gray-bold|
|dgru|dark-gray-underline|
|dgri|dark-gray-italic|
|dgrd|dark-gray-dimmed|
|dgrr|dark-gray-reverse|
|dgrbl|dark-gray-blink|
|dgrst|dark-gray-strike|

### `"#hex"` format
---

The "#hex" format is one way you typically see colors represented. It's simply the `#` character followed by 6 characters. The first two are for `red`, the second two are for `green`, and the third two are for `blue`. It's important that this string be surrounded in quotes, otherwise nushell thinks it's a commented out string.

Example: The primary `red` color is `"#ff0000"` or `"#FF0000"`. Upper and lower case in letters shouldn't make a difference.

This `"#hex"` format allows us to specify 24-bit truecolor tones to different parts of nushell.

## `full "#hex"` format
---
The `full "#hex"` format is a take on the `"#hex"` format but allows one to specify the foreground, background, and attributes in one line.

Example: `{ fg: "#ff0000" bg: "#0000ff" attr: b }`

* foreground of red in "#hex" format 
* background of blue in "#hex" format 
* attribute of bold abbreviated

## `Primitive values`
___

Primitive values are things like `int` and `string`. Primitive values and flatshapes can be set with a variety of color symbologies seen above.

This is the current list of primitives. Not all of these are configurable. The configurable ones are marked with *.

| primitive | default color | configurable |
| - | - | - |
| `any`|| |
| `binary`|Color::White.normal()| * |
| `block`|Color::White.normal()| * |
| `bool`|Color::White.normal()| * |
| `cellpath`|Color::White.normal()| * |
| `condition`|| |
| `custom`||  |
| `date`|Color::White.normal()| * |
| `duration`|Color::White.normal()| * |
| `expression`|| |
| `filesize`|Color::White.normal()| * |
| `float`|Color::White.normal()| * |
| `glob`|| |
| `import`|| |
| `int`|Color::White.normal()| * |
| `list`|Color::White.normal()| * |
| `nothing`|Color::White.normal()| * |
| `number`|| |
| `operator`|| |
| `path`|| |
| `range`|Color::White.normal()| * |
| `record`|Color::White.normal()| * |
| `signature`|| |
| `string`|Color::White.normal()| * |
| `table`|| |
| `var`|| |
| `vardecl`|| |
| `variable`|| |

#### special "primitives" (not really primitives but they exist solely for coloring)

| primitive | default color | configurable |
| - | - | - |
| `leading-trailing-space-bg`|Color::Rgb(128, 128, 128))| *|
| `header`|Color::Green.bold()| *|
| `empty`|Color::Blue.normal()| *|
| `row-index`|Color::Green.bold()| *|
| `hints`|Color::DarkGray.normal()| *|

Here's a small example of changing some of these values.
```
let config = {
    color-config: {
        separator: purple
        leading-trailing-space-bg: "#ffffff"
        header: gb
        date: wd
        filesize: c
        row-index: cb
        bool: red
        int: green
        duration: blue-bold
        range: purple
        float: red
        string: white
        nothing: red
        binary: red
        cellpath: cyan
        hints: dark-gray
    }
}
```
Here's another small example using multiple color syntaxes with some comments.
```
let config = {
    color-config: {
        separator: "#88b719" # this sets only the foreground color like PR #486
        leading-trailing-space-bg: white # this sets only the foreground color in the original style
        header: { # this is like PR #489
        fg: "#B01455", # note, quotes are required on the values with hex colors
        bg: "#ffb900",# note, commas are not required, it could also be all on one line
        attr: bli # note, there are no quotes around this value. it works with or without quotes
        }
        date: "#75507B"
        filesize: "#729fcf"
        row-index: { # note, that this is another way to set only the foreground, no need to specify bg and attr
        fg: "#e50914"
    }
}
```

## `FlatShape` values

As mentioned above, `flatshape` is a term used to indicate the sytax coloring.

Here's the current list of flat shapes.

| flatshape | default style | configurable |
| - | - | - |
| `flatshape-block`| fg(Color::Blue).bold()| * |
| `flatshape-bool`| fg(Color::LightCyan)| * |
| `flatshape-custom`| bold()| * |
| `flatshape-external`| fg(Color::Cyan)| * |
| `flatshape-externalarg`| fg(Color::Green).bold()| * |
| `flatshape-filepath`| fg(Color::Cyan)| * |
| `flatshape-flag`| fg(Color::Blue).bold()| * |
| `flatshape-float`|fg(Color::Purple).bold() | * |
| `flatshape-garbage`| fg(Color::White).on(Color::Red).bold()| * |
| `flatshape-globpattern`| fg(Color::Cyan).bold()| * |
| `flatshape-int`|fg(Color::Purple).bold() | * |
| `flatshape-internalcall`| fg(Color::Cyan).bold()| * |
| `flatshape-list`| fg(Color::Cyan).bold()| * |
| `flatshape-literal`| fg(Color::Blue)| * |
| `flatshape-nothing`| fg(Color::LightCyan)| * |
| `flatshape-operator`| fg(Color::Yellow)| * |
| `flatshape-range`| fg(Color::Yellow).bold()| * |
| `flatshape-record`| fg(Color::Cyan).bold()| * |
| `flatshape-signature`| fg(Color::Green).bold()| * |
| `flatshape-string`| fg(Color::Green)| * |
| `flatshape-string-interpolation`| fg(Color::Cyan).bold()| * |
| `flatshape-table`| fg(Color::Blue).bold()| * |
| `flatshape-variable`| fg(Color::Purple)| * |

Here's a small example of how to apply color to these items. Anything not specified will receive the default color.

```
let $config = {
    color-config: {
        flatshape-garbage: { fg: "#FFFFFF" bg: "#FF0000" attr: b}
        flatshape-bool: green
        flatshape-int: { fg: "#0000ff" attr: b}
    }
}
```

## `Prompt` configuration and coloring

The nushell prompt is configurable through these environment variables settings.

* `PROMPT_COMMAND`: Code to execute for setting up the prompt (block)
* `PROMPT_COMMAND_RIGHT`: Code to execute for setting up the *RIGHT* prompt (block) (see oh-my.nu in nu_scripts)
* `PROMPT_INDICATOR` = "〉": The indicator printed after the prompt (by default ">"-like Unicode symbol)
* `PROMPT_INDICATOR_VI_INSERT` = ": "
* `PROMPT_INDICATOR_VI_NORMAL` = "v "
* `PROMPT_MULTILINE_INDICATOR` = "::: "

Example: For a simple prompt one could do this. Note that `PROMPT_COMMAND` requires a `block` whereas the others require a `string`.

`> let-env PROMPT_COMMAND = { build-string (date now | date format '%m/%d/%Y %I:%M:%S%.3f') ': ' (pwd | path basename) }`

If you don't like the default `PROMPT_INDICATOR` you could change it like this.

`> let-env PROMPT_INDICATOR = "> "`

Coloring of the prompt is controlled by the `block` in `PROMPT_COMMAND` where you can write your own custom prompt. We've written a slightly fancy one that has git statuses located in the [nu_scripts repo](https://github.com/nushell/nu_scripts/blob/main/engine-q/prompt/oh-my.nu).

## `LS_COLORS` colors for the `ls` command

Nushell will respect and use the `LS_COLORS` environment variable setting on Mac, Linux, and Windows. This setting allows you to define the color of file types when you do a `ls`. For instance, you can make directories one color, *.md markdown files another color, *.toml files yet another color, etc. There are a variety of ways to color your file types.

There's an exhaustive list [here](https://github.com/trapd00r/LS_COLORS), which is overkill, but gives you an rudimentary understanding of how to create a ls_colors file that `dircolors` can turn into a `LS_COLORS` environment variable.

[This](https://www.linuxhowto.net/how-to-set-colors-for-ls-command/) is a pretty good introduction to `LS_COLORS`. I'm sure you can fine many more tutorials on the web.

I like the `vivid` application and currently have it configured in my `config.nu` like this. You can find `vivid` [here](https://github.com/sharkdp/vivid).

`let-env LS_COLORS = (vivid generate molokai | decode utf-8 | str trim)`

## Theming

Theming combines all the coloring above. Here's a quick example of one we put together quickly to demonstrate the ability to theme. This is a spin on the `base16` themes that we see so widespread on the web.

The key to making theming work is to make sure you specify all themes and colors you're going to use in the `config.nu` file *before* you declare the `let config = ` line.

```
# lets define some colors

let base00 = "#181818" # Default Background
let base01 = "#282828" # Lighter Background (Used for status bars, line number and folding marks)
let base02 = "#383838" # Selection Background
let base03 = "#585858" # Comments, Invisibles, Line Highlighting
let base04 = "#b8b8b8" # Dark Foreground (Used for status bars)
let base05 = "#d8d8d8" # Default Foreground, Caret, Delimiters, Operators
let base06 = "#e8e8e8" # Light Foreground (Not often used)
let base07 = "#f8f8f8" # Light Background (Not often used)
let base08 = "#ab4642" # Variables, XML Tags, Markup Link Text, Markup Lists, Diff Deleted
let base09 = "#dc9656" # Integers, Boolean, Constants, XML Attributes, Markup Link Url
let base0a = "#f7ca88" # Classes, Markup Bold, Search Text Background
let base0b = "#a1b56c" # Strings, Inherited Class, Markup Code, Diff Inserted
let base0c = "#86c1b9" # Support, Regular Expressions, Escape Characters, Markup Quotes
let base0d = "#7cafc2" # Functions, Methods, Attribute IDs, Headings
let base0e = "#ba8baf" # Keywords, Storage, Selector, Markup Italic, Diff Changed
let base0f = "#a16946" # Deprecated, Opening/Closing Embedded Language Tags, e.g. <?php ?>

# we're creating a theme here that uses the colors we defined above.

let base16-theme = {
    separator: $base03
    leading-trailing-space-bg: $base04
    header: $base0b
    date: $base0e
    filesize: $base0d
    row-index: $base0c
    bool: $base08
    int: $base0b
    duration: $base08
    range: $base08
    float: $base08
    string: $base04
    nothing: $base08
    binary: $base08
    cellpath: $base08
    hints: dark-gray

    # flatshape-garbage: { fg: $base07 bg: $base08 attr: b} # base16 white on red
    # but i like the regular white on red for parse errors
    flatshape-garbage: { fg: "#FFFFFF" bg: "#FF0000" attr: b}
    flatshape-bool: $base0d
    flatshape-int: { fg: $base0e attr: b}
    flatshape-float: { fg: $base0e attr: b}
    flatshape-range: { fg: $base0a attr: b}
    flatshape-internalcall: { fg: $base0c attr: b}
    flatshape-external: $base0c
    flatshape-externalarg: { fg: $base0b attr: b}
    flatshape-literal: $base0d
    flatshape-operator: $base0a
    flatshape-signature: { fg: $base0b attr: b}
    flatshape-string: $base0b
    flatshape-filepath: $base0d
    flatshape-globpattern: { fg: $base0d attr: b}
    flatshape-variable: $base0e
    flatshape-flag: { fg: $base0d attr: b}
    flatshape-custom: {attr: b}
}

# now let's apply our regular config settings but also apply the "color-config:" theme that we specified above.

let config = {
  filesize-metric: $true
  table-mode: rounded # basic, compact, compact-double, light, thin, with-love, rounded, reinforced, heavy, none, other
  use-ls-colors: $true
  color-config: $base16-theme # <-- this is the theme
  use-grid-icons: $true
  footer-mode: always #always, never, number-of-rows, auto
  animate-prompt: $false
  float-precision: 2
  without-color: $false
  filesize-format: "b" # b, kb, kib, mb, mib, gb, gib, tb, tib, pb, pib, eb, eib, zb, zib, auto
  edit-mode: emacs # vi
  max-history-size: 10000
  log-level: error
}
```
if you want to go full-tilt on theming, you'll want to theme all the items I mentioned at the very beginning, including LS_COLORS, and the prompt.  Good luck!
