# textview config

The configuration for textview, which is used to autoview text files, uses [bat](https://docs.rs/bat/0.15.4/bat/struct.PrettyPrinter.html). The textview configuration will **not** use any existing `bat` configuration you may have.

## Configuration Points and Defaults

| config point | definition | implemented |
| - | - | - |
| term_width | The character width of the terminal (default: autodetect) | yes |
| tab_width | The width of tab characters (default: None - do not turn tabs to spaces) | yes |
| colored_output | Whether or not the output should be colorized (default: true) | yes |
| true_color | Whether or not to output 24bit colors (default: true) | yes |
| header | Whether to show a header with the file name | yes |
| line_numbers | Whether to show line numbers | yes |
| grid | Whether to paint a grid, separating line numbers, git changes and the code | yes |
| vcs_modification_markers | Whether to show modification markers for VCS changes. This has no effect if the git feature is not activated. | yes |
| snip | Whether to show "snip" markers between visible line ranges (default: no) | yes |
| wrapping_mode | Text wrapping mode (default: do not wrap), options (Character, NoWrapping) | yes |
| use_italics | Whether or not to use ANSI italics (default: off) | yes |
| paging_mode | If and how to use a pager (default: no paging), options (Always, QuitIfOneScreen, Never) | yes |
| pager | Specify the command to start the pager (default: use "less") | yes |
| line_ranges | Specify the lines that should be printed (default: all) | no |
| highlight | Specify a line that should be highlighted (default: none). This can be called multiple times to highlight more than one line. See also: highlight_range. | no |
| highlight_range | Specify a range of lines that should be highlighted (default: none). This can be called multiple times to highlight more than one range of lines. | no |
| theme | Specify the highlighting theme (default: OneHalfDark) | yes |

## Example textview configuration for `config.toml`

```toml
[textview]
term_width = "default"
tab_width = 4
colored_output = true
true_color = true
header = true
line_numbers = false
grid = false
vcs_modification_markers = true
snip = true
wrapping_mode = "NoWrapping"
use_italics = true
paging_mode = "QuitIfOneScreen"
pager = "less"
theme = "TwoDark"
```

## Example Usage

```shell
> open src/main.rs
```

```shell
> cat some_file.txt | textview
```

```shell
> fetch https://www.jonathanturner.org/feed.xml --raw
```

## Help

For a more detailed description of the configuration points that textview uses, please visit the `bat` repo at <https://github.com/sharkdp/bat>.
