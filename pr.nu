# emulate the python rich print
# test case 1: pr "hello world"
# test case 2: pr "hello world" --style bold
# test case 3: pr "hello world" --style 'bold underline'
# test case 4: pr "hello world" --style 'dim italic red'
def pr [
  msg,
  --style: string,
] {
    let style = if $style == null {
        ""
    } else {
        parse_style $style
    }

    # embedded style is like
    # "now [bold]is[/bold] the time"
    let msg_with_embedded_style = parse_embedded_style $msg $style
    # print $"($msg_with_embedded_style | debug -r)"
    # print $"style: ($style)" | debug -r
    print $"($style)($msg_with_embedded_style)(ansi reset)"
}

def parse_style [ style_string ] {
    let span = (metadata $style_string).span
    # print $"style_string: ($style_string)"
    let style_list = $style_string | split row " "
    # print $"style_list: ($style_list)"
    mut ansi_style = ""
    for style in $style_list {
        # print $"matching style: `($style)`"
        # let span = (metadata $style).span
        let cur_style = match $style {
            "bold" => "\e[1m"
            "dim" => "\e[2m"
            "italic" => "\e[3m"
            "underline" => "\e[4m"
            "blink" => "\e[5m"
            "reverse" => "\e[7m"
            "hidden" => "\e[8m"
            "strike" => "\e[9m"
            "black" => "\e[30m"
            "red" => "\e[31m"
            "green" => "\e[32m"
            "yellow" => "\e[33m"
            "blue" => "\e[34m"
            "magenta" => "\e[35m"
            "purple" => "\e[35m"
            "cyan" => "\e[36m"
            "white" => "\e[37m"
            "default" | "normal" => "\e[39m"
            _ => { 
                error make { 
                    msg: $"unknown style ($style)", 
                    label: { 
                        text: "this" 
                        start: $span.start 
                        end: $span.end
                    }
                    } 
                }
        }
        # print $"cur_style: ($cur_style | debug -r)"
        $ansi_style = $ansi_style + $cur_style
    }
    $ansi_style
}

def parse_embedded_style [ msg, style ] {
    mut new_msg = $msg
    let span = (metadata $new_msg).span
    # find all the [ and ] in the string
    # let open_bracket = "["
    # let close_bracket = "]"
    # let's use binary since we can do --all
    # not really sure what the best way to do this is with unicode strings
    # we may want to add a `str index-of blah --all` parameter because then
    # we could use graphemes or utf-8-bytes
    # find all open brackets
    let open_bracket_indices = $new_msg | into binary | bytes index-of 0x[5b] --all
    # find all close brackets
    let close_bracket_indices = $new_msg | into binary | bytes index-of 0x[5d] --all

    if ($open_bracket_indices | length) != ($close_bracket_indices | length) {
        error make {
            msg: "unbalanced brackets"
            label: {
                text: "this"
                start: $span.start
                end: $span.end
            }
        }
    }

    # let's get what's between the indices

    # let's make pairs
    let pairs = $open_bracket_indices | zip $close_bracket_indices
    for item in ($pairs | reverse) {
        # get the whole thing like [bold] or [/bold]
        let attr = $new_msg | str substring ($item.0)..($item.1 + 1)
        # print $"attr: ($attr)"
        if ($attr | str starts-with "[/") {
            # print $"this is a closing tag"
            # this is a closing tag
            # we don't need to do anything
            if $style != "" {
                $new_msg = ($new_msg | str replace $attr $style)
            } else {
                $new_msg = ($new_msg | str replace $attr "\e[0m")
            }
            # print $"new_msg[/: ($new_msg | debug -r)"
        } else if ($attr | str starts-with "[") {
            # print $"this is an opening tag"
            # this is an embedded style
            let attr_len = $attr | str length
            let style = $attr | str substring 1..($attr_len - 1)
            let ansi_style = parse_style $style
            # replace with ansi reset then ansi style so previous styles don't bleed over
            $new_msg = ($new_msg | str replace $attr $"(ansi reset)($ansi_style)")
            # print $"new_msg[: ($new_msg | debug -r)"
        }
    }

    $new_msg
}

    #     self,
    #     *objects: Any,
    #     sep: str = " ",
    #     end: str = "\n",
    #     style: Optional[Union[str, Style]] = None,
    #     justify: Optional[JustifyMethod] = None,
    #     overflow: Optional[OverflowMethod] = None,
    #     no_wrap: Optional[bool] = None,
    #     emoji: Optional[bool] = None,
    #     markup: Optional[bool] = None,
    #     highlight: Optional[bool] = None,
    #     width: Optional[int] = None,
    #     height: Optional[int] = None,
    #     crop: bool = True,
    #     soft_wrap: Optional[bool] = None,
    #     new_line_start: bool = False,
    # ) -> None:
    #     """Print to the console.

    #     Args:
    #         objects (positional args): Objects to log to the terminal.
    #         sep (str, optional): String to write between print data. Defaults to " ".
    #         end (str, optional): String to write at end of print data. Defaults to "\\\\n".
    #         style (Union[str, Style], optional): A style to apply to output. Defaults to None.
    #         justify (str, optional): Justify method: "default", "left", "right", "center", or "full". Defaults to ``None``.
    #         overflow (str, optional): Overflow method: "ignore", "crop", "fold", or "ellipsis". Defaults to None.
    #         no_wrap (Optional[bool], optional): Disable word wrapping. Defaults to None.
    #         emoji (Optional[bool], optional): Enable emoji code, or ``None`` to use console default. Defaults to ``None``.
    #         markup (Optional[bool], optional): Enable markup, or ``None`` to use console default. Defaults to ``None``.
    #         highlight (Optional[bool], optional): Enable automatic highlighting, or ``None`` to use console default. Defaults to ``None``.
    #         width (Optional[int], optional): Width of output, or ``None`` to auto-detect. Defaults to ``None``.
    #         crop (Optional[bool], optional): Crop output to width of terminal. Defaults to True.
    #         soft_wrap (bool, optional): Enable soft wrap mode which disables word wrapping and cropping of text or ``None`` for
    #             Console default. Defaults to ``None``.
    #         new_line_start (bool, False): Insert a new line at the start if the output contains more than one line. Defaults to ``False``.
    #     """
