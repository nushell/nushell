# Coloring and Theming in Nushell

There are a few main parts that nushell allows you to change the color. All of these can be set in the `config.nu` configuration file. If you see the hash/hashtag/pound mark `#` in the config file it means the text after it is commented out.

1. table borders
2. primitive values
3. flatshapes (this is the command line syntax)
4. prompt
5. LS_COLORS

## `Table borders`
___

Table borders are controlled by the `table_mode` setting in the `config.nu`. Here is an example:
```
let $config = {
    table_mode: rounded
}
```

Here are the current options for `table_mode`:
1. `rounded` # of course, this is the best one :)
2. `basic`
3. `compact`
4. `compact_double`
5. `light`
6. `thin`
7. `with_love`
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
* `red_bold` - normal color red with bold attribute
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
|gb|green_bold|
|gu|green_underline|
|gi|green_italic|
|gd|green_dimmed|
|gr|green_reverse|
|gbl|green_blink|
|gst|green_strike|
|lg|light_green|
|lgb|light_green_bold|
|lgu|light_green_underline|
|lgi|light_green_italic|
|lgd|light_green_dimmed|
|lgr|light_green_reverse|
|lgbl|light_green_blink|
|lgst|light_green_strike|
|r|red|
|rb|red_bold|
|ru|red_underline|
|ri|red_italic|
|rd|red_dimmed|
|rr|red_reverse|
|rbl|red_blink|
|rst|red_strike|
|lr|light_red|
|lrb|light_red_bold|
|lru|light_red_underline|
|lri|light_red_italic|
|lrd|light_red_dimmed|
|lrr|light_red_reverse|
|lrbl|light_red_blink|
|lrst|light_red_strike|
|u|blue|
|ub|blue_bold|
|uu|blue_underline|
|ui|blue_italic|
|ud|blue_dimmed|
|ur|blue_reverse|
|ubl|blue_blink|
|ust|blue_strike|
|lu|light_blue|
|lub|light_blue_bold|
|luu|light_blue_underline|
|lui|light_blue_italic|
|lud|light_blue_dimmed|
|lur|light_blue_reverse|
|lubl|light_blue_blink|
|lust|light_blue_strike|
|b|black|
|bb|black_bold|
|bu|black_underline|
|bi|black_italic|
|bd|black_dimmed|
|br|black_reverse|
|bbl|black_blink|
|bst|black_strike|
|ligr|light_gray|
|ligrb|light_gray_bold|
|ligru|light_gray_underline|
|ligri|light_gray_italic|
|ligrd|light_gray_dimmed|
|ligrr|light_gray_reverse|
|ligrbl|light_gray_blink|
|ligrst|light_gray_strike|
|y|yellow|
|yb|yellow_bold|
|yu|yellow_underline|
|yi|yellow_italic|
|yd|yellow_dimmed|
|yr|yellow_reverse|
|ybl|yellow_blink|
|yst|yellow_strike|
|ly|light_yellow|
|lyb|light_yellow_bold|
|lyu|light_yellow_underline|
|lyi|light_yellow_italic|
|lyd|light_yellow_dimmed|
|lyr|light_yellow_reverse|
|lybl|light_yellow_blink|
|lyst|light_yellow_strike|
|p|purple|
|pb|purple_bold|
|pu|purple_underline|
|pi|purple_italic|
|pd|purple_dimmed|
|pr|purple_reverse|
|pbl|purple_blink|
|pst|purple_strike|
|lp|light_purple|
|lpb|light_purple_bold|
|lpu|light_purple_underline|
|lpi|light_purple_italic|
|lpd|light_purple_dimmed|
|lpr|light_purple_reverse|
|lpbl|light_purple_blink|
|lpst|light_purple_strike|
|c|cyan|
|cb|cyan_bold|
|cu|cyan_underline|
|ci|cyan_italic|
|cd|cyan_dimmed|
|cr|cyan_reverse|
|cbl|cyan_blink|
|cst|cyan_strike|
|lc|light_cyan|
|lcb|light_cyan_bold|
|lcu|light_cyan_underline|
|lci|light_cyan_italic|
|lcd|light_cyan_dimmed|
|lcr|light_cyan_reverse|
|lcbl|light_cyan_blink|
|lcst|light_cyan_strike|
|w|white|
|wb|white_bold|
|wu|white_underline|
|wi|white_italic|
|wd|white_dimmed|
|wr|white_reverse|
|wbl|white_blink|
|wst|white_strike|
|dgr|dark_gray|
|dgrb|dark_gray_bold|
|dgru|dark_gray_underline|
|dgri|dark_gray_italic|
|dgrd|dark_gray_dimmed|
|dgrr|dark_gray_reverse|
|dgrbl|dark_gray_blink|
|dgrst|dark_gray_strike|

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
| `leading_trailing_space_bg`|Color::Rgb(128, 128, 128))| *|
| `header`|Color::Green.bold()| *|
| `empty`|Color::Blue.normal()| *|
| `row_index`|Color::Green.bold()| *|
| `hints`|Color::DarkGray.normal()| *|

Here's a small example of changing some of these values.
```
let config = {
    color_config: {
        separator: purple
        leading_trailing_space_bg: "#ffffff"
        header: gb
        date: wd
        filesize: c
        row_index: cb
        bool: red
        int: green
        duration: blue_bold
        range: purple
        float: red
        string: white
        nothing: red
        binary: red
        cellpath: cyan
        hints: dark_gray
    }
}
```
Here's another small example using multiple color syntaxes with some comments.
```
let config = {
    color_config: {
        separator: "#88b719" # this sets only the foreground color like PR #486
        leading_trailing_space_bg: white # this sets only the foreground color in the original style
        header: { # this is like PR #489
        fg: "#B01455", # note, quotes are required on the values with hex colors
        bg: "#ffb900",# note, commas are not required, it could also be all on one line
        attr: bli # note, there are no quotes around this value. it works with or without quotes
        }
        date: "#75507B"
        filesize: "#729fcf"
        row_index: { # note, that this is another way to set only the foreground, no need to specify bg and attr
        fg: "#e50914"
    }
}
```

## `FlatShape` values

As mentioned above, `flatshape` is a term used to indicate the sytax coloring.

Here's the current list of flat shapes.

| flatshape | default style | configurable |
| - | - | - |
| `flatshape_block`| fg(Color::Blue).bold()| * |
| `flatshape_bool`| fg(Color::LightCyan)| * |
| `flatshape_custom`| bold()| * |
| `flatshape_external`| fg(Color::Cyan)| * |
| `flatshape_externalarg`| fg(Color::Green).bold()| * |
| `flatshape_filepath`| fg(Color::Cyan)| * |
| `flatshape_flag`| fg(Color::Blue).bold()| * |
| `flatshape_float`|fg(Color::Purple).bold() | * |
| `flatshape_garbage`| fg(Color::White).on(Color::Red).bold()| * |
| `flatshape_globpattern`| fg(Color::Cyan).bold()| * |
| `flatshape_int`|fg(Color::Purple).bold() | * |
| `flatshape_internalcall`| fg(Color::Cyan).bold()| * |
| `flatshape_list`| fg(Color::Cyan).bold()| * |
| `flatshape_literal`| fg(Color::Blue)| * |
| `flatshape_nothing`| fg(Color::LightCyan)| * |
| `flatshape_operator`| fg(Color::Yellow)| * |
| `flatshape_range`| fg(Color::Yellow).bold()| * |
| `flatshape_record`| fg(Color::Cyan).bold()| * |
| `flatshape_signature`| fg(Color::Green).bold()| * |
| `flatshape_string`| fg(Color::Green)| * |
| `flatshape_string_interpolation`| fg(Color::Cyan).bold()| * |
| `flatshape_table`| fg(Color::Blue).bold()| * |
| `flatshape_variable`| fg(Color::Purple)| * |

Here's a small example of how to apply color to these items. Anything not specified will receive the default color.

```
let $config = {
    color_config: {
        flatshape_garbage: { fg: "#FFFFFF" bg: "#FF0000" attr: b}
        flatshape_bool: green
        flatshape_int: { fg: "#0000ff" attr: b}
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

let base16_theme = {
    separator: $base03
    leading_trailing_space_bg: $base04
    header: $base0b
    date: $base0e
    filesize: $base0d
    row_index: $base0c
    bool: $base08
    int: $base0b
    duration: $base08
    range: $base08
    float: $base08
    string: $base04
    nothing: $base08
    binary: $base08
    cellpath: $base08
    hints: dark_gray

    # flatshape_garbage: { fg: $base07 bg: $base08 attr: b} # base16 white on red
    # but i like the regular white on red for parse errors
    flatshape_garbage: { fg: "#FFFFFF" bg: "#FF0000" attr: b}
    flatshape_bool: $base0d
    flatshape_int: { fg: $base0e attr: b}
    flatshape_float: { fg: $base0e attr: b}
    flatshape_range: { fg: $base0a attr: b}
    flatshape_internalcall: { fg: $base0c attr: b}
    flatshape_external: $base0c
    flatshape_externalarg: { fg: $base0b attr: b}
    flatshape_literal: $base0d
    flatshape_operator: $base0a
    flatshape_signature: { fg: $base0b attr: b}
    flatshape_string: $base0b
    flatshape_filepath: $base0d
    flatshape_globpattern: { fg: $base0d attr: b}
    flatshape_variable: $base0e
    flatshape_flag: { fg: $base0d attr: b}
    flatshape_custom: {attr: b}
}

# now let's apply our regular config settings but also apply the "color_config:" theme that we specified above.

let config = {
  filesize_metric: $true
  table_mode: rounded # basic, compact, compact_double, light, thin, with_love, rounded, reinforced, heavy, none, other
  use_ls_colors: $true
  color_config: $base16_theme # <-- this is the theme
  use_grid_icons: $true
  footer_mode: always #always, never, number_of_rows, auto
  animate_prompt: $false
  float_precision: 2
  without_color: $false
  filesize_format: "b" # b, kb, kib, mb, mib, gb, gib, tb, tib, pb, pib, eb, eib, zb, zib, auto
  edit_mode: emacs # vi
  max_history_size: 10000
  log_level: error
}
```
if you want to go full-tilt on theming, you'll want to theme all the items I mentioned at the very beginning, including LS_COLORS, and the prompt.  Good luck!
