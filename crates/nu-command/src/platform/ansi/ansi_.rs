use nu_ansi_term::*;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call, engine::Command, Category, Example, IntoInterruptiblePipelineData, IntoPipelineData,
    PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};
use once_cell::sync::Lazy;
use std::collections::HashMap;

#[derive(Clone)]
pub struct AnsiCommand;

struct AnsiCode {
    short_name: Option<&'static str>,
    long_name: &'static str,
    code: String,
}

#[rustfmt::skip]
static CODE_LIST: Lazy<Vec<AnsiCode>> = Lazy::new(|| { vec![
    AnsiCode{ short_name: Some("g"), long_name: "green", code: Color::Green.prefix().to_string()},
    AnsiCode{ short_name: Some("gb"), long_name: "green_bold", code: Color::Green.bold().prefix().to_string()},
    AnsiCode{ short_name: Some("gu"), long_name: "green_underline", code: Color::Green.underline().prefix().to_string()},
    AnsiCode{ short_name: Some("gi"), long_name: "green_italic", code: Color::Green.italic().prefix().to_string()},
    AnsiCode{ short_name: Some("gd"), long_name: "green_dimmed", code: Color::Green.dimmed().prefix().to_string()},
    AnsiCode{ short_name: Some("gr"), long_name: "green_reverse", code: Color::Green.reverse().prefix().to_string()},
    AnsiCode{ short_name: Some("bg_g"), long_name: "bg_green", code: Style::new().on(Color::Green).prefix().to_string()},

    AnsiCode{ short_name: Some("lg"), long_name: "light_green", code: Color::LightGreen.prefix().to_string()},
    AnsiCode{ short_name: Some("lgb"), long_name: "light_green_bold", code: Color::LightGreen.bold().prefix().to_string()},
    AnsiCode{ short_name: Some("lgu"), long_name: "light_green_underline", code: Color::LightGreen.underline().prefix().to_string()},
    AnsiCode{ short_name: Some("lgi"), long_name: "light_green_italic", code: Color::LightGreen.italic().prefix().to_string()},
    AnsiCode{ short_name: Some("lgd"), long_name: "light_green_dimmed", code: Color::LightGreen.dimmed().prefix().to_string()},
    AnsiCode{ short_name: Some("lgr"), long_name: "light_green_reverse", code: Color::LightGreen.reverse().prefix().to_string()},
    AnsiCode{ short_name: Some("bg_lg"), long_name: "bg_light_green", code: Style::new().on(Color::LightGreen).prefix().to_string()},

    AnsiCode{ short_name: Some("r"), long_name: "red", code: Color::Red.prefix().to_string()},
    AnsiCode{ short_name: Some("rb"), long_name: "red_bold", code: Color::Red.bold().prefix().to_string()},
    AnsiCode{ short_name: Some("ru"), long_name: "red_underline", code: Color::Red.underline().prefix().to_string()},
    AnsiCode{ short_name: Some("ri"), long_name: "red_italic", code: Color::Red.italic().prefix().to_string()},
    AnsiCode{ short_name: Some("rd"), long_name: "red_dimmed", code: Color::Red.dimmed().prefix().to_string()},
    AnsiCode{ short_name: Some("rr"), long_name: "red_reverse", code: Color::Red.reverse().prefix().to_string()},
    AnsiCode{ short_name: Some("bg_r"), long_name: "bg_red", code: Style::new().on(Color::Red).prefix().to_string()},

    AnsiCode{ short_name: Some("lr"), long_name: "light_red", code: Color::LightRed.prefix().to_string()},
    AnsiCode{ short_name: Some("lrb"), long_name: "light_red_bold", code: Color::LightRed.bold().prefix().to_string()},
    AnsiCode{ short_name: Some("lru"), long_name: "light_red_underline", code: Color::LightRed.underline().prefix().to_string()},
    AnsiCode{ short_name: Some("lri"), long_name: "light_red_italic", code: Color::LightRed.italic().prefix().to_string()},
    AnsiCode{ short_name: Some("lrd"), long_name: "light_red_dimmed", code: Color::LightRed.dimmed().prefix().to_string()},
    AnsiCode{ short_name: Some("lrr"), long_name: "light_red_reverse", code: Color::LightRed.reverse().prefix().to_string()},
    AnsiCode{ short_name: Some("bg_lr"), long_name: "bg_light_red", code: Style::new().on(Color::LightRed).prefix().to_string()},

    AnsiCode{ short_name: Some("u"), long_name: "blue", code: Color::Blue.prefix().to_string()},
    AnsiCode{ short_name: Some("ub"), long_name: "blue_bold", code: Color::Blue.bold().prefix().to_string()},
    AnsiCode{ short_name: Some("uu"), long_name: "blue_underline", code: Color::Blue.underline().prefix().to_string()},
    AnsiCode{ short_name: Some("ui"), long_name: "blue_italic", code: Color::Blue.italic().prefix().to_string()},
    AnsiCode{ short_name: Some("ud"), long_name: "blue_dimmed", code: Color::Blue.dimmed().prefix().to_string()},
    AnsiCode{ short_name: Some("ur"), long_name: "blue_reverse", code: Color::Blue.reverse().prefix().to_string()},
    AnsiCode{ short_name: Some("bg_u"), long_name: "bg_blue", code: Style::new().on(Color::Blue).prefix().to_string()},

    AnsiCode{ short_name: Some("lu"), long_name: "light_blue", code: Color::LightBlue.prefix().to_string()},
    AnsiCode{ short_name: Some("lub"), long_name: "light_blue_bold", code: Color::LightBlue.bold().prefix().to_string()},
    AnsiCode{ short_name: Some("luu"), long_name: "light_blue_underline", code: Color::LightBlue.underline().prefix().to_string()},
    AnsiCode{ short_name: Some("lui"), long_name: "light_blue_italic", code: Color::LightBlue.italic().prefix().to_string()},
    AnsiCode{ short_name: Some("lud"), long_name: "light_blue_dimmed", code: Color::LightBlue.dimmed().prefix().to_string()},
    AnsiCode{ short_name: Some("lur"), long_name: "light_blue_reverse", code: Color::LightBlue.reverse().prefix().to_string()},
    AnsiCode{ short_name: Some("bg_lu"), long_name: "bg_light_blue", code: Style::new().on(Color::LightBlue).prefix().to_string()},

    AnsiCode{ short_name: Some("b"), long_name: "black", code: Color::Black.prefix().to_string()},
    AnsiCode{ short_name: Some("bb"), long_name: "black_bold", code: Color::Black.bold().prefix().to_string()},
    AnsiCode{ short_name: Some("bu"), long_name: "black_underline", code: Color::Black.underline().prefix().to_string()},
    AnsiCode{ short_name: Some("bi"), long_name: "black_italic", code: Color::Black.italic().prefix().to_string()},
    AnsiCode{ short_name: Some("bd"), long_name: "black_dimmed", code: Color::Black.dimmed().prefix().to_string()},
    AnsiCode{ short_name: Some("br"), long_name: "black_reverse", code: Color::Black.reverse().prefix().to_string()},
    AnsiCode{ short_name: Some("bg_b"), long_name: "bg_black", code: Style::new().on(Color::Black).prefix().to_string()},

    AnsiCode{ short_name: Some("ligr"), long_name: "light_gray", code: Color::LightGray.prefix().to_string()},
    AnsiCode{ short_name: Some("ligrb"), long_name: "light_gray_bold", code: Color::LightGray.bold().prefix().to_string()},
    AnsiCode{ short_name: Some("ligru"), long_name: "light_gray_underline", code: Color::LightGray.underline().prefix().to_string()},
    AnsiCode{ short_name: Some("ligri"), long_name: "light_gray_italic", code: Color::LightGray.italic().prefix().to_string()},
    AnsiCode{ short_name: Some("ligrd"), long_name: "light_gray_dimmed", code: Color::LightGray.dimmed().prefix().to_string()},
    AnsiCode{ short_name: Some("ligrr"), long_name: "light_gray_reverse", code: Color::LightGray.reverse().prefix().to_string()},
    AnsiCode{ short_name: Some("bg_ligr"), long_name: "bg_light_gray", code: Style::new().on(Color::LightGray).prefix().to_string()},

    AnsiCode{ short_name: Some("y"), long_name: "yellow", code: Color::Yellow.prefix().to_string()},
    AnsiCode{ short_name: Some("yb"), long_name: "yellow_bold", code: Color::Yellow.bold().prefix().to_string()},
    AnsiCode{ short_name: Some("yu"), long_name: "yellow_underline", code: Color::Yellow.underline().prefix().to_string()},
    AnsiCode{ short_name: Some("yi"), long_name: "yellow_italic", code: Color::Yellow.italic().prefix().to_string()},
    AnsiCode{ short_name: Some("yd"), long_name: "yellow_dimmed", code: Color::Yellow.dimmed().prefix().to_string()},
    AnsiCode{ short_name: Some("yr"), long_name: "yellow_reverse", code: Color::Yellow.reverse().prefix().to_string()},
    AnsiCode{ short_name: Some("bg_y"), long_name: "bg_yellow", code: Style::new().on(Color::Yellow).prefix().to_string()},

    AnsiCode{ short_name: Some("ly"), long_name: "light_yellow", code: Color::LightYellow.prefix().to_string()},
    AnsiCode{ short_name: Some("lyb"), long_name: "light_yellow_bold", code: Color::LightYellow.bold().prefix().to_string()},
    AnsiCode{ short_name: Some("lyu"), long_name: "light_yellow_underline", code: Color::LightYellow.underline().prefix().to_string()},
    AnsiCode{ short_name: Some("lyi"), long_name: "light_yellow_italic", code: Color::LightYellow.italic().prefix().to_string()},
    AnsiCode{ short_name: Some("lyd"), long_name: "light_yellow_dimmed", code: Color::LightYellow.dimmed().prefix().to_string()},
    AnsiCode{ short_name: Some("lyr"), long_name: "light_yellow_reverse", code: Color::LightYellow.reverse().prefix().to_string()},
    AnsiCode{ short_name: Some("bg_ly"), long_name: "bg_light_yellow", code: Style::new().on(Color::LightYellow).prefix().to_string()},

    AnsiCode{ short_name: Some("p"), long_name: "purple", code: Color::Purple.prefix().to_string()},
    AnsiCode{ short_name: Some("pb"), long_name: "purple_bold", code: Color::Purple.bold().prefix().to_string()},
    AnsiCode{ short_name: Some("pu"), long_name: "purple_underline", code: Color::Purple.underline().prefix().to_string()},
    AnsiCode{ short_name: Some("pi"), long_name: "purple_italic", code: Color::Purple.italic().prefix().to_string()},
    AnsiCode{ short_name: Some("pd"), long_name: "purple_dimmed", code: Color::Purple.dimmed().prefix().to_string()},
    AnsiCode{ short_name: Some("pr"), long_name: "purple_reverse", code: Color::Purple.reverse().prefix().to_string()},
    AnsiCode{ short_name: Some("bg_p"), long_name: "bg_purple", code: Style::new().on(Color::Purple).prefix().to_string()},

    AnsiCode{ short_name: Some("lp"), long_name: "light_purple", code: Color::LightPurple.prefix().to_string()},
    AnsiCode{ short_name: Some("lpb"), long_name: "light_purple_bold", code: Color::LightPurple.bold().prefix().to_string()},
    AnsiCode{ short_name: Some("lpu"), long_name: "light_purple_underline", code: Color::LightPurple.underline().prefix().to_string()},
    AnsiCode{ short_name: Some("lpi"), long_name: "light_purple_italic", code: Color::LightPurple.italic().prefix().to_string()},
    AnsiCode{ short_name: Some("lpd"), long_name: "light_purple_dimmed", code: Color::LightPurple.dimmed().prefix().to_string()},
    AnsiCode{ short_name: Some("lpr"), long_name: "light_purple_reverse", code: Color::LightPurple.reverse().prefix().to_string()},
    AnsiCode{ short_name: Some("bg_lp"), long_name: "bg_light_purple", code: Style::new().on(Color::LightPurple).prefix().to_string()},

    AnsiCode{ short_name: Some("c"), long_name: "cyan", code: Color::Cyan.prefix().to_string()},
    AnsiCode{ short_name: Some("cb"), long_name: "cyan_bold", code: Color::Cyan.bold().prefix().to_string()},
    AnsiCode{ short_name: Some("cu"), long_name: "cyan_underline", code: Color::Cyan.underline().prefix().to_string()},
    AnsiCode{ short_name: Some("ci"), long_name: "cyan_italic", code: Color::Cyan.italic().prefix().to_string()},
    AnsiCode{ short_name: Some("cd"), long_name: "cyan_dimmed", code: Color::Cyan.dimmed().prefix().to_string()},
    AnsiCode{ short_name: Some("cr"), long_name: "cyan_reverse", code: Color::Cyan.reverse().prefix().to_string()},
    AnsiCode{ short_name: Some("bg_c"), long_name: "bg_cyan", code: Style::new().on(Color::Cyan).prefix().to_string()},

    AnsiCode{ short_name: Some("lc"), long_name: "light_cyan", code: Color::LightCyan.prefix().to_string()},
    AnsiCode{ short_name: Some("lcb"), long_name: "light_cyan_bold", code: Color::LightCyan.bold().prefix().to_string()},
    AnsiCode{ short_name: Some("lcu"), long_name: "light_cyan_underline", code: Color::LightCyan.underline().prefix().to_string()},
    AnsiCode{ short_name: Some("lci"), long_name: "light_cyan_italic", code: Color::LightCyan.italic().prefix().to_string()},
    AnsiCode{ short_name: Some("lcd"), long_name: "light_cyan_dimmed", code: Color::LightCyan.dimmed().prefix().to_string()},
    AnsiCode{ short_name: Some("lcr"), long_name: "light_cyan_reverse", code: Color::LightCyan.reverse().prefix().to_string()},
    AnsiCode{ short_name: Some("bg_lc"), long_name: "bg_light_cyan", code: Style::new().on(Color::LightCyan).prefix().to_string()},

    AnsiCode{ short_name: Some("w"), long_name: "white", code: Color::White.prefix().to_string()},
    AnsiCode{ short_name: Some("wb"), long_name: "white_bold", code: Color::White.bold().prefix().to_string()},
    AnsiCode{ short_name: Some("wu"), long_name: "white_underline", code: Color::White.underline().prefix().to_string()},
    AnsiCode{ short_name: Some("wi"), long_name: "white_italic", code: Color::White.italic().prefix().to_string()},
    AnsiCode{ short_name: Some("wd"), long_name: "white_dimmed", code: Color::White.dimmed().prefix().to_string()},
    AnsiCode{ short_name: Some("wr"), long_name: "white_reverse", code: Color::White.reverse().prefix().to_string()},
    AnsiCode{ short_name: Some("bg_w"), long_name: "bg_white", code: Style::new().on(Color::White).prefix().to_string()},

    AnsiCode{ short_name: Some("dgr"), long_name: "dark_gray", code: Color::DarkGray.prefix().to_string()},
    AnsiCode{ short_name: Some("dgrb"), long_name: "dark_gray_bold", code: Color::DarkGray.bold().prefix().to_string()},
    AnsiCode{ short_name: Some("dgru"), long_name: "dark_gray_underline", code: Color::DarkGray.underline().prefix().to_string()},
    AnsiCode{ short_name: Some("dgri"), long_name: "dark_gray_italic", code: Color::DarkGray.italic().prefix().to_string()},
    AnsiCode{ short_name: Some("dgrd"), long_name: "dark_gray_dimmed", code: Color::DarkGray.dimmed().prefix().to_string()},
    AnsiCode{ short_name: Some("dgrr"), long_name: "dark_gray_reverse", code: Color::DarkGray.reverse().prefix().to_string()},
    AnsiCode{ short_name: Some("bg_dgr"), long_name: "bg_dark_gray", code: Style::new().on(Color::DarkGray).prefix().to_string()},

    AnsiCode{ short_name: Some("def"), long_name: "default", code: Color::Default.prefix().to_string()},
    AnsiCode{ short_name: Some("defb"), long_name: "default_bold", code: Color::Default.bold().prefix().to_string()},
    AnsiCode{ short_name: Some("defu"), long_name: "default_underline", code: Color::Default.underline().prefix().to_string()},
    AnsiCode{ short_name: Some("defi"), long_name: "default_italic", code: Color::Default.italic().prefix().to_string()},
    AnsiCode{ short_name: Some("defd"), long_name: "default_dimmed", code: Color::Default.dimmed().prefix().to_string()},
    AnsiCode{ short_name: Some("defr"), long_name: "default_reverse", code: Color::Default.reverse().prefix().to_string()},
    AnsiCode{ short_name: Some("bg_def"), long_name: "bg_default", code: Style::new().on(Color::Default).prefix().to_string()},

    // Xterm 256 colors with conflicting names names preceeded by x
    AnsiCode { short_name: Some("xblack"), long_name: "xterm_black", code: Color::Fixed(0).prefix().to_string()},
    AnsiCode { short_name: Some("maroon"), long_name: "xterm_maroon", code: Color::Fixed(1).prefix().to_string()},
    AnsiCode { short_name: Some("xgreen"), long_name: "xterm_green", code: Color::Fixed(2).prefix().to_string()},
    AnsiCode { short_name: Some("olive"), long_name: "xterm_olive", code: Color::Fixed(3).prefix().to_string()},
    AnsiCode { short_name: Some("navy"), long_name: "xterm_navy", code: Color::Fixed(4).prefix().to_string()},
    AnsiCode { short_name: Some("xpurplea"), long_name: "xterm_purplea", code: Color::Fixed(5).prefix().to_string()},
    AnsiCode { short_name: Some("teal"), long_name: "xterm_teal", code: Color::Fixed(6).prefix().to_string()},
    AnsiCode { short_name: Some("silver"), long_name: "xterm_silver", code: Color::Fixed(7).prefix().to_string()},
    AnsiCode { short_name: Some("grey"), long_name: "xterm_grey", code: Color::Fixed(8).prefix().to_string()},
    AnsiCode { short_name: Some("xred"), long_name: "xterm_red", code: Color::Fixed(9).prefix().to_string()},
    AnsiCode { short_name: Some("lime"), long_name: "xterm_lime", code: Color::Fixed(10).prefix().to_string()},
    AnsiCode { short_name: Some("xyellow"), long_name: "xterm_yellow", code: Color::Fixed(11).prefix().to_string()},
    AnsiCode { short_name: Some("xblue"), long_name: "xterm_blue", code: Color::Fixed(12).prefix().to_string()},
    AnsiCode { short_name: Some("fuchsia"), long_name: "xterm_fuchsia", code: Color::Fixed(13).prefix().to_string()},
    AnsiCode { short_name: Some("aqua"), long_name: "xterm_aqua", code: Color::Fixed(14).prefix().to_string()},
    AnsiCode { short_name: Some("xwhite"), long_name: "xterm_white", code: Color::Fixed(15).prefix().to_string()},
    AnsiCode { short_name: Some("grey0"), long_name: "xterm_grey0", code: Color::Fixed(16).prefix().to_string()},
    AnsiCode { short_name: Some("navyblue"), long_name: "xterm_navyblue", code: Color::Fixed(17).prefix().to_string()},
    AnsiCode { short_name: Some("darkblue"), long_name: "xterm_darkblue", code: Color::Fixed(18).prefix().to_string()},
    AnsiCode { short_name: Some("blue3a"), long_name: "xterm_blue3a", code: Color::Fixed(19).prefix().to_string()},
    AnsiCode { short_name: Some("blue3b"), long_name: "xterm_blue3b", code: Color::Fixed(20).prefix().to_string()},
    AnsiCode { short_name: Some("blue1"), long_name: "xterm_blue1", code: Color::Fixed(21).prefix().to_string()},
    AnsiCode { short_name: Some("darkgreen"), long_name: "xterm_darkgreen", code: Color::Fixed(22).prefix().to_string()},
    AnsiCode { short_name: Some("deepskyblue4a"), long_name: "xterm_deepskyblue4a", code: Color::Fixed(23).prefix().to_string()},
    AnsiCode { short_name: Some("deepskyblue4b"), long_name: "xterm_deepskyblue4b", code: Color::Fixed(24).prefix().to_string()},
    AnsiCode { short_name: Some("deepskyblue4c"), long_name: "xterm_deepskyblue4c", code: Color::Fixed(25).prefix().to_string()},
    AnsiCode { short_name: Some("dodgerblue3"), long_name: "xterm_dodgerblue3", code: Color::Fixed(26).prefix().to_string()},
    AnsiCode { short_name: Some("dodgerblue2"), long_name: "xterm_dodgerblue2", code: Color::Fixed(27).prefix().to_string()},
    AnsiCode { short_name: Some("green4"), long_name: "xterm_green4", code: Color::Fixed(28).prefix().to_string()},
    AnsiCode { short_name: Some("springgreen4"), long_name: "xterm_springgreen4", code: Color::Fixed(29).prefix().to_string()},
    AnsiCode { short_name: Some("turquoise4"), long_name: "xterm_turquoise4", code: Color::Fixed(30).prefix().to_string()},
    AnsiCode { short_name: Some("deepskyblue3a"), long_name: "xterm_deepskyblue3a", code: Color::Fixed(31).prefix().to_string()},
    AnsiCode { short_name: Some("deepskyblue3b"), long_name: "xterm_deepskyblue3b", code: Color::Fixed(32).prefix().to_string()},
    AnsiCode { short_name: Some("dodgerblue1"), long_name: "xterm_dodgerblue1", code: Color::Fixed(33).prefix().to_string()},
    AnsiCode { short_name: Some("green3a"), long_name: "xterm_green3a", code: Color::Fixed(34).prefix().to_string()},
    AnsiCode { short_name: Some("springgreen3a"), long_name: "xterm_springgreen3a", code: Color::Fixed(35).prefix().to_string()},
    AnsiCode { short_name: Some("darkcyan"), long_name: "xterm_darkcyan", code: Color::Fixed(36).prefix().to_string()},
    AnsiCode { short_name: Some("lightseagreen"), long_name: "xterm_lightseagreen", code: Color::Fixed(37).prefix().to_string()},
    AnsiCode { short_name: Some("deepskyblue2"), long_name: "xterm_deepskyblue2", code: Color::Fixed(38).prefix().to_string()},
    AnsiCode { short_name: Some("deepskyblue1"), long_name: "xterm_deepskyblue1", code: Color::Fixed(39).prefix().to_string()},
    AnsiCode { short_name: Some("green3b"), long_name: "xterm_green3b", code: Color::Fixed(40).prefix().to_string()},
    AnsiCode { short_name: Some("springgreen3b"), long_name: "xterm_springgreen3b", code: Color::Fixed(41).prefix().to_string()},
    AnsiCode { short_name: Some("springgreen2a"), long_name: "xterm_springgreen2a", code: Color::Fixed(42).prefix().to_string()},
    AnsiCode { short_name: Some("cyan3"), long_name: "xterm_cyan3", code: Color::Fixed(43).prefix().to_string()},
    AnsiCode { short_name: Some("darkturquoise"), long_name: "xterm_darkturquoise", code: Color::Fixed(44).prefix().to_string()},
    AnsiCode { short_name: Some("turquoise2"), long_name: "xterm_turquoise2", code: Color::Fixed(45).prefix().to_string()},
    AnsiCode { short_name: Some("green1"), long_name: "xterm_green1", code: Color::Fixed(46).prefix().to_string()},
    AnsiCode { short_name: Some("springgreen2b"), long_name: "xterm_springgreen2b", code: Color::Fixed(47).prefix().to_string()},
    AnsiCode { short_name: Some("springgreen1"), long_name: "xterm_springgreen1", code: Color::Fixed(48).prefix().to_string()},
    AnsiCode { short_name: Some("mediumspringgreen"), long_name: "xterm_mediumspringgreen", code: Color::Fixed(49).prefix().to_string()},
    AnsiCode { short_name: Some("cyan2"), long_name: "xterm_cyan2", code: Color::Fixed(50).prefix().to_string()},
    AnsiCode { short_name: Some("cyan1"), long_name: "xterm_cyan1", code: Color::Fixed(51).prefix().to_string()},
    AnsiCode { short_name: Some("darkreda"), long_name: "xterm_darkreda", code: Color::Fixed(52).prefix().to_string()},
    AnsiCode { short_name: Some("deeppink4a"), long_name: "xterm_deeppink4a", code: Color::Fixed(53).prefix().to_string()},
    AnsiCode { short_name: Some("purple4a"), long_name: "xterm_purple4a", code: Color::Fixed(54).prefix().to_string()},
    AnsiCode { short_name: Some("purple4b"), long_name: "xterm_purple4b", code: Color::Fixed(55).prefix().to_string()},
    AnsiCode { short_name: Some("purple3"), long_name: "xterm_purple3", code: Color::Fixed(56).prefix().to_string()},
    AnsiCode { short_name: Some("blueviolet"), long_name: "xterm_blueviolet", code: Color::Fixed(57).prefix().to_string()},
    AnsiCode { short_name: Some("orange4a"), long_name: "xterm_orange4a", code: Color::Fixed(58).prefix().to_string()},
    AnsiCode { short_name: Some("grey37"), long_name: "xterm_grey37", code: Color::Fixed(59).prefix().to_string()},
    AnsiCode { short_name: Some("mediumpurple4"), long_name: "xterm_mediumpurple4", code: Color::Fixed(60).prefix().to_string()},
    AnsiCode { short_name: Some("slateblue3a"), long_name: "xterm_slateblue3a", code: Color::Fixed(61).prefix().to_string()},
    AnsiCode { short_name: Some("slateblue3b"), long_name: "xterm_slateblue3b", code: Color::Fixed(62).prefix().to_string()},
    AnsiCode { short_name: Some("royalblue1"), long_name: "xterm_royalblue1", code: Color::Fixed(63).prefix().to_string()},
    AnsiCode { short_name: Some("chartreuse4"), long_name: "xterm_chartreuse4", code: Color::Fixed(64).prefix().to_string()},
    AnsiCode { short_name: Some("darkseagreen4a"), long_name: "xterm_darkseagreen4a", code: Color::Fixed(65).prefix().to_string()},
    AnsiCode { short_name: Some("paleturquoise4"), long_name: "xterm_paleturquoise4", code: Color::Fixed(66).prefix().to_string()},
    AnsiCode { short_name: Some("steelblue"), long_name: "xterm_steelblue", code: Color::Fixed(67).prefix().to_string()},
    AnsiCode { short_name: Some("steelblue3"), long_name: "xterm_steelblue3", code: Color::Fixed(68).prefix().to_string()},
    AnsiCode { short_name: Some("cornflowerblue"), long_name: "xterm_cornflowerblue", code: Color::Fixed(69).prefix().to_string()},
    AnsiCode { short_name: Some("chartreuse3a"), long_name: "xterm_chartreuse3a", code: Color::Fixed(70).prefix().to_string()},
    AnsiCode { short_name: Some("darkseagreen4b"), long_name: "xterm_darkseagreen4b", code: Color::Fixed(71).prefix().to_string()},
    AnsiCode { short_name: Some("cadetbluea"), long_name: "xterm_cadetbluea", code: Color::Fixed(72).prefix().to_string()},
    AnsiCode { short_name: Some("cadetblueb"), long_name: "xterm_cadetblueb", code: Color::Fixed(73).prefix().to_string()},
    AnsiCode { short_name: Some("skyblue3"), long_name: "xterm_skyblue3", code: Color::Fixed(74).prefix().to_string()},
    AnsiCode { short_name: Some("steelblue1a"), long_name: "xterm_steelblue1a", code: Color::Fixed(75).prefix().to_string()},
    AnsiCode { short_name: Some("chartreuse3b"), long_name: "xterm_chartreuse3b", code: Color::Fixed(76).prefix().to_string()},
    AnsiCode { short_name: Some("palegreen3a"), long_name: "xterm_palegreen3a", code: Color::Fixed(77).prefix().to_string()},
    AnsiCode { short_name: Some("seagreen3"), long_name: "xterm_seagreen3", code: Color::Fixed(78).prefix().to_string()},
    AnsiCode { short_name: Some("aquamarine3"), long_name: "xterm_aquamarine3", code: Color::Fixed(79).prefix().to_string()},
    AnsiCode { short_name: Some("mediumturquoise"), long_name: "xterm_mediumturquoise", code: Color::Fixed(80).prefix().to_string()},
    AnsiCode { short_name: Some("steelblue1b"), long_name: "xterm_steelblue1b", code: Color::Fixed(81).prefix().to_string()},
    AnsiCode { short_name: Some("chartreuse2a"), long_name: "xterm_chartreuse2a", code: Color::Fixed(82).prefix().to_string()},
    AnsiCode { short_name: Some("seagreen2"), long_name: "xterm_seagreen2", code: Color::Fixed(83).prefix().to_string()},
    AnsiCode { short_name: Some("seagreen1a"), long_name: "xterm_seagreen1a", code: Color::Fixed(84).prefix().to_string()},
    AnsiCode { short_name: Some("seagreen1b"), long_name: "xterm_seagreen1b", code: Color::Fixed(85).prefix().to_string()},
    AnsiCode { short_name: Some("aquamarine1a"), long_name: "xterm_aquamarine1a", code: Color::Fixed(86).prefix().to_string()},
    AnsiCode { short_name: Some("darkslategray2"), long_name: "xterm_darkslategray2", code: Color::Fixed(87).prefix().to_string()},
    AnsiCode { short_name: Some("darkredb"), long_name: "xterm_darkredb", code: Color::Fixed(88).prefix().to_string()},
    AnsiCode { short_name: Some("deeppink4b"), long_name: "xterm_deeppink4b", code: Color::Fixed(89).prefix().to_string()},
    AnsiCode { short_name: Some("darkmagentaa"), long_name: "xterm_darkmagentaa", code: Color::Fixed(90).prefix().to_string()},
    AnsiCode { short_name: Some("darkmagentab"), long_name: "xterm_darkmagentab", code: Color::Fixed(91).prefix().to_string()},
    AnsiCode { short_name: Some("darkvioleta"), long_name: "xterm_darkvioleta", code: Color::Fixed(92).prefix().to_string()},
    AnsiCode { short_name: Some("xpurpleb"), long_name: "xterm_purpleb", code: Color::Fixed(93).prefix().to_string()},
    AnsiCode { short_name: Some("orange4b"), long_name: "xterm_orange4b", code: Color::Fixed(94).prefix().to_string()},
    AnsiCode { short_name: Some("lightpink4"), long_name: "xterm_lightpink4", code: Color::Fixed(95).prefix().to_string()},
    AnsiCode { short_name: Some("plum4"), long_name: "xterm_plum4", code: Color::Fixed(96).prefix().to_string()},
    AnsiCode { short_name: Some("mediumpurple3a"), long_name: "xterm_mediumpurple3a", code: Color::Fixed(97).prefix().to_string()},
    AnsiCode { short_name: Some("mediumpurple3b"), long_name: "xterm_mediumpurple3b", code: Color::Fixed(98).prefix().to_string()},
    AnsiCode { short_name: Some("slateblue1"), long_name: "xterm_slateblue1", code: Color::Fixed(99).prefix().to_string()},
    AnsiCode { short_name: Some("yellow4a"), long_name: "xterm_yellow4a", code: Color::Fixed(100).prefix().to_string()},
    AnsiCode { short_name: Some("wheat4"), long_name: "xterm_wheat4", code: Color::Fixed(101).prefix().to_string()},
    AnsiCode { short_name: Some("grey53"), long_name: "xterm_grey53", code: Color::Fixed(102).prefix().to_string()},
    AnsiCode { short_name: Some("lightslategrey"), long_name: "xterm_lightslategrey", code: Color::Fixed(103).prefix().to_string()},
    AnsiCode { short_name: Some("mediumpurple"), long_name: "xterm_mediumpurple", code: Color::Fixed(104).prefix().to_string()},
    AnsiCode { short_name: Some("lightslateblue"), long_name: "xterm_lightslateblue", code: Color::Fixed(105).prefix().to_string()},
    AnsiCode { short_name: Some("yellow4b"), long_name: "xterm_yellow4b", code: Color::Fixed(106).prefix().to_string()},
    AnsiCode { short_name: Some("darkolivegreen3a"), long_name: "xterm_darkolivegreen3a", code: Color::Fixed(107).prefix().to_string()},
    AnsiCode { short_name: Some("darkseagreen"), long_name: "xterm_darkseagreen", code: Color::Fixed(108).prefix().to_string()},
    AnsiCode { short_name: Some("lightskyblue3a"), long_name: "xterm_lightskyblue3a", code: Color::Fixed(109).prefix().to_string()},
    AnsiCode { short_name: Some("lightskyblue3b"), long_name: "xterm_lightskyblue3b", code: Color::Fixed(110).prefix().to_string()},
    AnsiCode { short_name: Some("skyblue2"), long_name: "xterm_skyblue2", code: Color::Fixed(111).prefix().to_string()},
    AnsiCode { short_name: Some("chartreuse2b"), long_name: "xterm_chartreuse2b", code: Color::Fixed(112).prefix().to_string()},
    AnsiCode { short_name: Some("darkolivegreen3b"), long_name: "xterm_darkolivegreen3b", code: Color::Fixed(113).prefix().to_string()},
    AnsiCode { short_name: Some("palegreen3b"), long_name: "xterm_palegreen3b", code: Color::Fixed(114).prefix().to_string()},
    AnsiCode { short_name: Some("darkseagreen3a"), long_name: "xterm_darkseagreen3a", code: Color::Fixed(115).prefix().to_string()},
    AnsiCode { short_name: Some("darkslategray3"), long_name: "xterm_darkslategray3", code: Color::Fixed(116).prefix().to_string()},
    AnsiCode { short_name: Some("skyblue1"), long_name: "xterm_skyblue1", code: Color::Fixed(117).prefix().to_string()},
    AnsiCode { short_name: Some("chartreuse1"), long_name: "xterm_chartreuse1", code: Color::Fixed(118).prefix().to_string()},
    AnsiCode { short_name: Some("lightgreena"), long_name: "xterm_lightgreena", code: Color::Fixed(119).prefix().to_string()},
    AnsiCode { short_name: Some("lightgreenb"), long_name: "xterm_lightgreenb", code: Color::Fixed(120).prefix().to_string()},
    AnsiCode { short_name: Some("palegreen1a"), long_name: "xterm_palegreen1a", code: Color::Fixed(121).prefix().to_string()},
    AnsiCode { short_name: Some("aquamarine1b"), long_name: "xterm_aquamarine1b", code: Color::Fixed(122).prefix().to_string()},
    AnsiCode { short_name: Some("darkslategray1"), long_name: "xterm_darkslategray1", code: Color::Fixed(123).prefix().to_string()},
    AnsiCode { short_name: Some("red3a"), long_name: "xterm_red3a", code: Color::Fixed(124).prefix().to_string()},
    AnsiCode { short_name: Some("deeppink4c"), long_name: "xterm_deeppink4c", code: Color::Fixed(125).prefix().to_string()},
    AnsiCode { short_name: Some("mediumvioletred"), long_name: "xterm_mediumvioletred", code: Color::Fixed(126).prefix().to_string()},
    AnsiCode { short_name: Some("magenta3"), long_name: "xterm_magenta3", code: Color::Fixed(127).prefix().to_string()},
    AnsiCode { short_name: Some("darkvioletb"), long_name: "xterm_darkvioletb", code: Color::Fixed(128).prefix().to_string()},
    AnsiCode { short_name: Some("xpurplec"), long_name: "xterm_purplec", code: Color::Fixed(129).prefix().to_string()},
    AnsiCode { short_name: Some("darkorange3a"), long_name: "xterm_darkorange3a", code: Color::Fixed(130).prefix().to_string()},
    AnsiCode { short_name: Some("indianreda"), long_name: "xterm_indianreda", code: Color::Fixed(131).prefix().to_string()},
    AnsiCode { short_name: Some("hotpink3a"), long_name: "xterm_hotpink3a", code: Color::Fixed(132).prefix().to_string()},
    AnsiCode { short_name: Some("mediumorchid3"), long_name: "xterm_mediumorchid3", code: Color::Fixed(133).prefix().to_string()},
    AnsiCode { short_name: Some("mediumorchid"), long_name: "xterm_mediumorchid", code: Color::Fixed(134).prefix().to_string()},
    AnsiCode { short_name: Some("mediumpurple2a"), long_name: "xterm_mediumpurple2a", code: Color::Fixed(135).prefix().to_string()},
    AnsiCode { short_name: Some("darkgoldenrod"), long_name: "xterm_darkgoldenrod", code: Color::Fixed(136).prefix().to_string()},
    AnsiCode { short_name: Some("lightsalmon3a"), long_name: "xterm_lightsalmon3a", code: Color::Fixed(137).prefix().to_string()},
    AnsiCode { short_name: Some("rosybrown"), long_name: "xterm_rosybrown", code: Color::Fixed(138).prefix().to_string()},
    AnsiCode { short_name: Some("grey63"), long_name: "xterm_grey63", code: Color::Fixed(139).prefix().to_string()},
    AnsiCode { short_name: Some("mediumpurple2b"), long_name: "xterm_mediumpurple2b", code: Color::Fixed(140).prefix().to_string()},
    AnsiCode { short_name: Some("mediumpurple1"), long_name: "xterm_mediumpurple1", code: Color::Fixed(141).prefix().to_string()},
    AnsiCode { short_name: Some("gold3a"), long_name: "xterm_gold3a", code: Color::Fixed(142).prefix().to_string()},
    AnsiCode { short_name: Some("darkkhaki"), long_name: "xterm_darkkhaki", code: Color::Fixed(143).prefix().to_string()},
    AnsiCode { short_name: Some("navajowhite3"), long_name: "xterm_navajowhite3", code: Color::Fixed(144).prefix().to_string()},
    AnsiCode { short_name: Some("grey69"), long_name: "xterm_grey69", code: Color::Fixed(145).prefix().to_string()},
    AnsiCode { short_name: Some("lightsteelblue3"), long_name: "xterm_lightsteelblue3", code: Color::Fixed(146).prefix().to_string()},
    AnsiCode { short_name: Some("lightsteelblue"), long_name: "xterm_lightsteelblue", code: Color::Fixed(147).prefix().to_string()},
    AnsiCode { short_name: Some("yellow3a"), long_name: "xterm_yellow3a", code: Color::Fixed(148).prefix().to_string()},
    AnsiCode { short_name: Some("darkolivegreen3c"), long_name: "xterm_darkolivegreen3c", code: Color::Fixed(149).prefix().to_string()},
    AnsiCode { short_name: Some("darkseagreen3b"), long_name: "xterm_darkseagreen3b", code: Color::Fixed(150).prefix().to_string()},
    AnsiCode { short_name: Some("darkseagreen2a"), long_name: "xterm_darkseagreen2a", code: Color::Fixed(151).prefix().to_string()},
    AnsiCode { short_name: Some("lightcyan3"), long_name: "xterm_lightcyan3", code: Color::Fixed(152).prefix().to_string()},
    AnsiCode { short_name: Some("lightskyblue1"), long_name: "xterm_lightskyblue1", code: Color::Fixed(153).prefix().to_string()},
    AnsiCode { short_name: Some("greenyellow"), long_name: "xterm_greenyellow", code: Color::Fixed(154).prefix().to_string()},
    AnsiCode { short_name: Some("darkolivegreen2"), long_name: "xterm_darkolivegreen2", code: Color::Fixed(155).prefix().to_string()},
    AnsiCode { short_name: Some("palegreen1b"), long_name: "xterm_palegreen1b", code: Color::Fixed(156).prefix().to_string()},
    AnsiCode { short_name: Some("darkseagreen2b"), long_name: "xterm_darkseagreen2b", code: Color::Fixed(157).prefix().to_string()},
    AnsiCode { short_name: Some("darkseagreen1a"), long_name: "xterm_darkseagreen1a", code: Color::Fixed(158).prefix().to_string()},
    AnsiCode { short_name: Some("paleturquoise1"), long_name: "xterm_paleturquoise1", code: Color::Fixed(159).prefix().to_string()},
    AnsiCode { short_name: Some("red3b"), long_name: "xterm_red3b", code: Color::Fixed(160).prefix().to_string()},
    AnsiCode { short_name: Some("deeppink3a"), long_name: "xterm_deeppink3a", code: Color::Fixed(161).prefix().to_string()},
    AnsiCode { short_name: Some("deeppink3b"), long_name: "xterm_deeppink3b", code: Color::Fixed(162).prefix().to_string()},
    AnsiCode { short_name: Some("magenta3a"), long_name: "xterm_magenta3a", code: Color::Fixed(163).prefix().to_string()},
    AnsiCode { short_name: Some("magenta3b"), long_name: "xterm_magenta3b", code: Color::Fixed(164).prefix().to_string()},
    AnsiCode { short_name: Some("magenta2a"), long_name: "xterm_magenta2a", code: Color::Fixed(165).prefix().to_string()},
    AnsiCode { short_name: Some("darkorange3b"), long_name: "xterm_darkorange3b", code: Color::Fixed(166).prefix().to_string()},
    AnsiCode { short_name: Some("indianredb"), long_name: "xterm_indianredb", code: Color::Fixed(167).prefix().to_string()},
    AnsiCode { short_name: Some("hotpink3b"), long_name: "xterm_hotpink3b", code: Color::Fixed(168).prefix().to_string()},
    AnsiCode { short_name: Some("hotpink2"), long_name: "xterm_hotpink2", code: Color::Fixed(169).prefix().to_string()},
    AnsiCode { short_name: Some("orchid"), long_name: "xterm_orchid", code: Color::Fixed(170).prefix().to_string()},
    AnsiCode { short_name: Some("mediumorchid1a"), long_name: "xterm_mediumorchid1a", code: Color::Fixed(171).prefix().to_string()},
    AnsiCode { short_name: Some("orange3"), long_name: "xterm_orange3", code: Color::Fixed(172).prefix().to_string()},
    AnsiCode { short_name: Some("lightsalmon3b"), long_name: "xterm_lightsalmon3b", code: Color::Fixed(173).prefix().to_string()},
    AnsiCode { short_name: Some("lightpink3"), long_name: "xterm_lightpink3", code: Color::Fixed(174).prefix().to_string()},
    AnsiCode { short_name: Some("pink3"), long_name: "xterm_pink3", code: Color::Fixed(175).prefix().to_string()},
    AnsiCode { short_name: Some("plum3"), long_name: "xterm_plum3", code: Color::Fixed(176).prefix().to_string()},
    AnsiCode { short_name: Some("violet"), long_name: "xterm_violet", code: Color::Fixed(177).prefix().to_string()},
    AnsiCode { short_name: Some("gold3b"), long_name: "xterm_gold3b", code: Color::Fixed(178).prefix().to_string()},
    AnsiCode { short_name: Some("lightgoldenrod3"), long_name: "xterm_lightgoldenrod3", code: Color::Fixed(179).prefix().to_string()},
    AnsiCode { short_name: Some("tan"), long_name: "xterm_tan", code: Color::Fixed(180).prefix().to_string()},
    AnsiCode { short_name: Some("mistyrose3"), long_name: "xterm_mistyrose3", code: Color::Fixed(181).prefix().to_string()},
    AnsiCode { short_name: Some("thistle3"), long_name: "xterm_thistle3", code: Color::Fixed(182).prefix().to_string()},
    AnsiCode { short_name: Some("plum2"), long_name: "xterm_plum2", code: Color::Fixed(183).prefix().to_string()},
    AnsiCode { short_name: Some("yellow3b"), long_name: "xterm_yellow3b", code: Color::Fixed(184).prefix().to_string()},
    AnsiCode { short_name: Some("khaki3"), long_name: "xterm_khaki3", code: Color::Fixed(185).prefix().to_string()},
    AnsiCode { short_name: Some("lightgoldenrod2"), long_name: "xterm_lightgoldenrod2", code: Color::Fixed(186).prefix().to_string()},
    AnsiCode { short_name: Some("lightyellow3"), long_name: "xterm_lightyellow3", code: Color::Fixed(187).prefix().to_string()},
    AnsiCode { short_name: Some("grey84"), long_name: "xterm_grey84", code: Color::Fixed(188).prefix().to_string()},
    AnsiCode { short_name: Some("lightsteelblue1"), long_name: "xterm_lightsteelblue1", code: Color::Fixed(189).prefix().to_string()},
    AnsiCode { short_name: Some("yellow2"), long_name: "xterm_yellow2", code: Color::Fixed(190).prefix().to_string()},
    AnsiCode { short_name: Some("darkolivegreen1a"), long_name: "xterm_darkolivegreen1a", code: Color::Fixed(191).prefix().to_string()},
    AnsiCode { short_name: Some("darkolivegreen1b"), long_name: "xterm_darkolivegreen1b", code: Color::Fixed(192).prefix().to_string()},
    AnsiCode { short_name: Some("darkseagreen1b"), long_name: "xterm_darkseagreen1b", code: Color::Fixed(193).prefix().to_string()},
    AnsiCode { short_name: Some("honeydew2"), long_name: "xterm_honeydew2", code: Color::Fixed(194).prefix().to_string()},
    AnsiCode { short_name: Some("lightcyan1"), long_name: "xterm_lightcyan1", code: Color::Fixed(195).prefix().to_string()},
    AnsiCode { short_name: Some("red1"), long_name: "xterm_red1", code: Color::Fixed(196).prefix().to_string()},
    AnsiCode { short_name: Some("deeppink2"), long_name: "xterm_deeppink2", code: Color::Fixed(197).prefix().to_string()},
    AnsiCode { short_name: Some("deeppink1a"), long_name: "xterm_deeppink1a", code: Color::Fixed(198).prefix().to_string()},
    AnsiCode { short_name: Some("deeppink1b"), long_name: "xterm_deeppink1b", code: Color::Fixed(199).prefix().to_string()},
    AnsiCode { short_name: Some("magenta2b"), long_name: "xterm_magenta2b", code: Color::Fixed(200).prefix().to_string()},
    AnsiCode { short_name: Some("magenta1"), long_name: "xterm_magenta1", code: Color::Fixed(201).prefix().to_string()},
    AnsiCode { short_name: Some("orangered1"), long_name: "xterm_orangered1", code: Color::Fixed(202).prefix().to_string()},
    AnsiCode { short_name: Some("indianred1a"), long_name: "xterm_indianred1a", code: Color::Fixed(203).prefix().to_string()},
    AnsiCode { short_name: Some("indianred1b"), long_name: "xterm_indianred1b", code: Color::Fixed(204).prefix().to_string()},
    AnsiCode { short_name: Some("hotpinka"), long_name: "xterm_hotpinka", code: Color::Fixed(205).prefix().to_string()},
    AnsiCode { short_name: Some("hotpinkb"), long_name: "xterm_hotpinkb", code: Color::Fixed(206).prefix().to_string()},
    AnsiCode { short_name: Some("mediumorchid1b"), long_name: "xterm_mediumorchid1b", code: Color::Fixed(207).prefix().to_string()},
    AnsiCode { short_name: Some("darkorange"), long_name: "xterm_darkorange", code: Color::Fixed(208).prefix().to_string()},
    AnsiCode { short_name: Some("salmon1"), long_name: "xterm_salmon1", code: Color::Fixed(209).prefix().to_string()},
    AnsiCode { short_name: Some("lightcoral"), long_name: "xterm_lightcoral", code: Color::Fixed(210).prefix().to_string()},
    AnsiCode { short_name: Some("palevioletred1"), long_name: "xterm_palevioletred1", code: Color::Fixed(211).prefix().to_string()},
    AnsiCode { short_name: Some("orchid2"), long_name: "xterm_orchid2", code: Color::Fixed(212).prefix().to_string()},
    AnsiCode { short_name: Some("orchid1"), long_name: "xterm_orchid1", code: Color::Fixed(213).prefix().to_string()},
    AnsiCode { short_name: Some("orange1"), long_name: "xterm_orange1", code: Color::Fixed(214).prefix().to_string()},
    AnsiCode { short_name: Some("sandybrown"), long_name: "xterm_sandybrown", code: Color::Fixed(215).prefix().to_string()},
    AnsiCode { short_name: Some("lightsalmon1"), long_name: "xterm_lightsalmon1", code: Color::Fixed(216).prefix().to_string()},
    AnsiCode { short_name: Some("lightpink1"), long_name: "xterm_lightpink1", code: Color::Fixed(217).prefix().to_string()},
    AnsiCode { short_name: Some("pink1"), long_name: "xterm_pink1", code: Color::Fixed(218).prefix().to_string()},
    AnsiCode { short_name: Some("plum1"), long_name: "xterm_plum1", code: Color::Fixed(219).prefix().to_string()},
    AnsiCode { short_name: Some("gold1"), long_name: "xterm_gold1", code: Color::Fixed(220).prefix().to_string()},
    AnsiCode { short_name: Some("lightgoldenrod2a"), long_name: "xterm_lightgoldenrod2a", code: Color::Fixed(221).prefix().to_string()},
    AnsiCode { short_name: Some("lightgoldenrod2b"), long_name: "xterm_lightgoldenrod2b", code: Color::Fixed(222).prefix().to_string()},
    AnsiCode { short_name: Some("navajowhite1"), long_name: "xterm_navajowhite1", code: Color::Fixed(223).prefix().to_string()},
    AnsiCode { short_name: Some("mistyrose1"), long_name: "xterm_mistyrose1", code: Color::Fixed(224).prefix().to_string()},
    AnsiCode { short_name: Some("thistle1"), long_name: "xterm_thistle1", code: Color::Fixed(225).prefix().to_string()},
    AnsiCode { short_name: Some("yellow1"), long_name: "xterm_yellow1", code: Color::Fixed(226).prefix().to_string()},
    AnsiCode { short_name: Some("lightgoldenrod1"), long_name: "xterm_lightgoldenrod1", code: Color::Fixed(227).prefix().to_string()},
    AnsiCode { short_name: Some("khaki1"), long_name: "xterm_khaki1", code: Color::Fixed(228).prefix().to_string()},
    AnsiCode { short_name: Some("wheat1"), long_name: "xterm_wheat1", code: Color::Fixed(229).prefix().to_string()},
    AnsiCode { short_name: Some("cornsilk1"), long_name: "xterm_cornsilk1", code: Color::Fixed(230).prefix().to_string()},
    AnsiCode { short_name: Some("grey100"), long_name: "xterm_grey100", code: Color::Fixed(231).prefix().to_string()},
    AnsiCode { short_name: Some("grey3"), long_name: "xterm_grey3", code: Color::Fixed(232).prefix().to_string()},
    AnsiCode { short_name: Some("grey7"), long_name: "xterm_grey7", code: Color::Fixed(233).prefix().to_string()},
    AnsiCode { short_name: Some("grey11"), long_name: "xterm_grey11", code: Color::Fixed(234).prefix().to_string()},
    AnsiCode { short_name: Some("grey15"), long_name: "xterm_grey15", code: Color::Fixed(235).prefix().to_string()},
    AnsiCode { short_name: Some("grey19"), long_name: "xterm_grey19", code: Color::Fixed(236).prefix().to_string()},
    AnsiCode { short_name: Some("grey23"), long_name: "xterm_grey23", code: Color::Fixed(237).prefix().to_string()},
    AnsiCode { short_name: Some("grey27"), long_name: "xterm_grey27", code: Color::Fixed(238).prefix().to_string()},
    AnsiCode { short_name: Some("grey30"), long_name: "xterm_grey30", code: Color::Fixed(239).prefix().to_string()},
    AnsiCode { short_name: Some("grey35"), long_name: "xterm_grey35", code: Color::Fixed(240).prefix().to_string()},
    AnsiCode { short_name: Some("grey39"), long_name: "xterm_grey39", code: Color::Fixed(241).prefix().to_string()},
    AnsiCode { short_name: Some("grey42"), long_name: "xterm_grey42", code: Color::Fixed(242).prefix().to_string()},
    AnsiCode { short_name: Some("grey46"), long_name: "xterm_grey46", code: Color::Fixed(243).prefix().to_string()},
    AnsiCode { short_name: Some("grey50"), long_name: "xterm_grey50", code: Color::Fixed(244).prefix().to_string()},
    AnsiCode { short_name: Some("grey54"), long_name: "xterm_grey54", code: Color::Fixed(245).prefix().to_string()},
    AnsiCode { short_name: Some("grey58"), long_name: "xterm_grey58", code: Color::Fixed(246).prefix().to_string()},
    AnsiCode { short_name: Some("grey62"), long_name: "xterm_grey62", code: Color::Fixed(247).prefix().to_string()},
    AnsiCode { short_name: Some("grey66"), long_name: "xterm_grey66", code: Color::Fixed(248).prefix().to_string()},
    AnsiCode { short_name: Some("grey70"), long_name: "xterm_grey70", code: Color::Fixed(249).prefix().to_string()},
    AnsiCode { short_name: Some("grey74"), long_name: "xterm_grey74", code: Color::Fixed(250).prefix().to_string()},
    AnsiCode { short_name: Some("grey78"), long_name: "xterm_grey78", code: Color::Fixed(251).prefix().to_string()},
    AnsiCode { short_name: Some("grey82"), long_name: "xterm_grey82", code: Color::Fixed(252).prefix().to_string()},
    AnsiCode { short_name: Some("grey85"), long_name: "xterm_grey85", code: Color::Fixed(253).prefix().to_string()},
    AnsiCode { short_name: Some("grey89"), long_name: "xterm_grey89", code: Color::Fixed(254).prefix().to_string()},
    AnsiCode { short_name: Some("grey93"), long_name: "xterm_grey93", code: Color::Fixed(255).prefix().to_string()},

    AnsiCode{ short_name: None, long_name: "reset", code: "\x1b[0m".to_owned()},

    // Attributes
    AnsiCode { short_name: Some("n"), long_name: "attr_normal", code: Color::Green.suffix().to_string()},
    AnsiCode { short_name: Some("bo"), long_name: "attr_bold", code: Style::new().bold().prefix().to_string()},
    AnsiCode { short_name: Some("d"), long_name: "attr_dimmed", code: Style::new().dimmed().prefix().to_string()},
    AnsiCode { short_name: Some("i"), long_name: "attr_italic", code: Style::new().italic().prefix().to_string()},
    AnsiCode { short_name: Some("u"), long_name: "attr_underline", code: Style::new().underline().prefix().to_string()},
    AnsiCode { short_name: Some("bl"), long_name: "attr_blink", code: Style::new().blink().prefix().to_string()},
    AnsiCode { short_name: Some("h"), long_name: "attr_hidden", code: Style::new().hidden().prefix().to_string()},
    AnsiCode { short_name: Some("s"), long_name: "attr_strike", code: Style::new().strikethrough().prefix().to_string()},

    // Reference for ansi codes https://gist.github.com/fnky/458719343aabd01cfb17a3a4f7296797
    // Another good reference http://ascii-table.com/ansi-escape-sequences.php

    // For setting title like `echo [(char title) (pwd) (char bel)] | str join`
    AnsiCode{short_name: None, long_name:"title", code: "\x1b]2;".to_string()}, // ESC]2; xterm sets window title using OSC syntax escapes

    // Ansi Erase Sequences
    AnsiCode{ short_name: None, long_name:"clear_screen", code: "\x1b[J".to_string()}, // clears the screen
    AnsiCode{ short_name: None, long_name:"clear_screen_from_cursor_to_end", code: "\x1b[0J".to_string()}, // clears from cursor until end of screen
    AnsiCode{ short_name: None, long_name:"clear_screen_from_cursor_to_beginning", code: "\x1b[1J".to_string()}, // clears from cursor to beginning of screen
    AnsiCode{ short_name: Some("cls"), long_name:"clear_entire_screen", code: "\x1b[2J".to_string()}, // clears the entire screen
    AnsiCode{ short_name: Some("clsb"), long_name:"clear_entire_screen_plus_buffer", code: "\x1b[3J".to_string()}, // clear entire screen and delete all lines saved in the scrollback buffer
    AnsiCode{ short_name: None, long_name:"erase_line", code: "\x1b[K".to_string()},                   // clears the current line
    AnsiCode{ short_name: None, long_name:"erase_line_from_cursor_to_end", code: "\x1b[0K".to_string()}, // clears from cursor to end of line
    AnsiCode{ short_name: None, long_name:"erase_line_from_cursor_to_beginning", code: "\x1b[1K".to_string()}, // clears from cursor to start of line
    AnsiCode{ short_name: None, long_name:"erase_entire_line", code: "\x1b[2K".to_string()},                   // clears entire line

    // Turn on/off cursor
    AnsiCode{ short_name: None, long_name:"cursor_off", code: "\x1b[?25l".to_string()},
    AnsiCode{ short_name: None, long_name:"cursor_on", code: "\x1b[?25h".to_string()},
    AnsiCode{ short_name: Some("home"), long_name:"cursor_home", code: "\x1b[H".to_string()},

    // Turn on/off blinking
    AnsiCode{ short_name: None, long_name:"cursor_blink_off", code: "\x1b[?12l".to_string()},
    AnsiCode{ short_name: None, long_name:"cursor_blink_on", code: "\x1b[?12h".to_string()},

    // Cursor position in ESC [ <r>;<c>R where r = row and c = column
    AnsiCode{ short_name: None, long_name:"cursor_position", code: "\x1b[6n".to_string()},

    // Report Terminal Identity
    AnsiCode{ short_name: None, long_name:"identity", code: "\x1b[0c".to_string()},

    // Ansi escape only - CSI command
    AnsiCode{ short_name: Some("esc"), long_name: "escape", code: "\x1b".to_string()},
    // Ansi escape only - CSI command
    AnsiCode{ short_name: Some("csi"), long_name: "escape_left", code: "\x1b[".to_string()},
    // OSC escape (Operating system command)
    AnsiCode{ short_name: Some("osc"), long_name:"escape_right", code: "\x1b]".to_string()},
    // OSC string terminator
    AnsiCode{ short_name: Some("st"), long_name:"string_terminator", code: "\x1b\\".to_string()},

    // Ansi Rgb - Needs to be 32;2;r;g;b or 48;2;r;g;b
    // assuming the rgb will be passed via command and no here
    AnsiCode{ short_name: None, long_name:"rgb_fg", code: "\x1b[38;2;".to_string()},
    AnsiCode{ short_name: None, long_name:"rgb_bg", code: "\x1b[48;2;".to_string()},

    // Ansi color index - Needs 38;5;idx or 48;5;idx where idx = 0 to 255
    AnsiCode{ short_name: Some("idx_fg"), long_name: "color_idx_fg", code: "\x1b[38;5;".to_string()},
    AnsiCode{ short_name: Some("idx_bg"), long_name:"color_idx_bg", code: "\x1b[48;5;".to_string()},

    // Returns terminal size like "[<r>;<c>R" where r is rows and c is columns
    // This should work assuming your terminal is not greater than 999x999
    AnsiCode{ short_name: None, long_name:"size", code: "\x1b[s\x1b[999;999H\x1b[6n\x1b[u".to_string()}
    ]
});

static CODE_MAP: Lazy<HashMap<&'static str, &'static str>> =
    Lazy::new(|| build_ansi_hashmap(&CODE_LIST));

impl Command for AnsiCommand {
    fn name(&self) -> &str {
        "ansi"
    }

    fn signature(&self) -> Signature {
        Signature::build("ansi")
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .optional(
                "code",
                SyntaxShape::Any,
                "the name of the code to use like 'green' or 'reset' to reset the color",
            )
            .switch(
                "escape", // \x1b[
                "escape sequence without the escape character(s)",
                Some('e'),
            )
            .switch(
                "osc", // \x1b]
                "operating system command (ocs) escape sequence without the escape character(s)",
                Some('o'),
            )
            .switch("list", "list available ansi code names", Some('l'))
            .category(Category::Platform)
    }

    fn usage(&self) -> &str {
        "Output ANSI codes to change color."
    }

    fn extra_usage(&self) -> &str {
        r#"For escape sequences:
Escape: '\x1b[' is not required for --escape parameter
Format: #(;#)m
Example: 1;31m for bold red or 2;37;41m for dimmed white fg with red bg
There can be multiple text formatting sequence numbers
separated by a ; and ending with an m where the # is of the
following values:
    attribute_number, abbreviation, description
    0     reset / normal display
    1  b  bold or increased intensity
    2  d  faint or decreased intensity
    3  i  italic on (non-mono font)
    4  u  underline on
    5  l  slow blink on
    6     fast blink on
    7  r  reverse video on
    8  h  nondisplayed (invisible) on
    9  s  strike-through on

    foreground/bright colors    background/bright colors
    30/90    black              40/100    black
    31/91    red                41/101    red
    32/92    green              42/102    green
    33/93    yellow             43/103    yellow
    34/94    blue               44/104    blue
    35/95    magenta            45/105    magenta
    36/96    cyan               46/106    cyan
    37/97    white              47/107    white
    39       default            49        default
    https://en.wikipedia.org/wiki/ANSI_escape_code

OSC: '\x1b]' is not required for --osc parameter
Example: echo [(ansi -o '0') 'some title' (char bel)] | str join
Format: #
    0 Set window title and icon name
    1 Set icon name
    2 Set window title
    4 Set/read color palette
    9 iTerm2 Grown notifications
    10 Set foreground color (x11 color spec)
    11 Set background color (x11 color spec)
    ... others"#
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Change color to green",
                example: r#"ansi green"#,
                result: Some(Value::test_string("\u{1b}[32m")),
            },
            Example {
                description: "Reset the color",
                example: r#"ansi reset"#,
                result: Some(Value::test_string("\u{1b}[0m")),
            },
            Example {
                description:
                    "Use ansi to color text (rb = red bold, gb = green bold, pb = purple bold)",
                example: r#"$'(ansi rb)Hello (ansi gb)Nu (ansi pb)World(ansi reset)'"#,
                result: Some(Value::test_string(
                    "\u{1b}[1;31mHello \u{1b}[1;32mNu \u{1b}[1;35mWorld\u{1b}[0m",
                )),
            },
            Example {
                description: "Use ansi to color text (italic bright yellow on red 'Hello' with green bold 'Nu' and purple bold 'World')",
                example: r#"[(ansi -e '3;93;41m') Hello (ansi reset) " " (ansi gb) Nu " " (ansi pb) World (ansi reset)] | str join"#,
                result: None,
                // Test disabled because the final expression in the pipeline is
                // not the command being tested, and this violated assumptions
                // made by the run-time input/output type-checking tests.
                // result: Some(Value::test_string(
                //     "\u{1b}[3;93;41mHello\u{1b}[0m \u{1b}[1;32mNu \u{1b}[1;35mWorld\u{1b}[0m",
                // )),
            },
            Example {
                description: "Use ansi to color text with a style (blue on red in bold)",
                example: r#"$"(ansi -e { fg: '#0000ff' bg: '#ff0000' attr: b })Hello Nu World(ansi reset)""#,
                result: Some(Value::test_string(
                    "\u{1b}[1;48;2;255;0;0;38;2;0;0;255mHello Nu World\u{1b}[0m",
                )),
            },
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["text-color", "text-style", "colors"]
    }

    fn run(
        &self,
        engine_state: &nu_protocol::engine::EngineState,
        stack: &mut nu_protocol::engine::Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        let list: bool = call.has_flag("list");
        let escape: bool = call.has_flag("escape");
        let osc: bool = call.has_flag("osc");
        let use_ansi_coloring = engine_state.get_config().use_ansi_coloring;

        if list {
            return generate_ansi_code_list(engine_state, call.head, use_ansi_coloring);
        }

        // The code can now be one of the ansi abbreviations like green_bold
        // or it can be a record like this: { fg: "#ff0000" bg: "#00ff00" attr: bli }
        // this record is defined in nu-color-config crate
        let code: Value = match call.opt(engine_state, stack, 0)? {
            Some(c) => c,
            None => return Err(ShellError::MissingParameter("code".into(), call.head)),
        };

        let param_is_string = matches!(code, Value::String { val: _, span: _ });

        if escape && osc {
            return Err(ShellError::IncompatibleParameters {
                left_message: "escape".into(),
                left_span: call
                    .get_named_arg("escape")
                    .expect("Unexpected missing argument")
                    .span,
                right_message: "osc".into(),
                right_span: call
                    .get_named_arg("osc")
                    .expect("Unexpected missing argument")
                    .span,
            });
        }

        let code_string = if param_is_string {
            code.as_string().expect("error getting code as string")
        } else {
            "".to_string()
        };

        let param_is_valid_string = param_is_string && !code_string.is_empty();

        if (escape || osc) && (param_is_valid_string) {
            let code_vec: Vec<char> = code_string.chars().collect();
            if code_vec[0] == '\\' {
                return Err(ShellError::UnsupportedInput(
                    String::from("no need for escape characters"),
                    call.get_flag_expr("escape")
                        .expect("Unexpected missing argument")
                        .span,
                ));
            }
        }

        let output = if escape && param_is_valid_string {
            format!("\x1b[{}", code_string)
        } else if osc && param_is_valid_string {
            // Operating system command aka osc  ESC ] <- note the right brace, not left brace for osc
            // OCS's need to end with either:
            // bel '\x07' char
            // string terminator aka st '\\' char
            format!("\x1b]{}", code_string)
        } else if param_is_valid_string {
            // parse hex colors like #00FF00
            if code_string.starts_with('#') {
                match nu_color_config::color_from_hex(&code_string) {
                    Ok(color) => match color {
                        Some(c) => c.prefix().to_string(),
                        None => Color::White.prefix().to_string(),
                    },
                    Err(err) => {
                        return Err(ShellError::GenericError(
                            "error parsing hex color".to_string(),
                            format!("{}", err),
                            Some(code.span()?),
                            None,
                            Vec::new(),
                        ));
                    }
                }
            } else {
                match str_to_ansi(&code_string) {
                    Some(c) => c,
                    None => {
                        return Err(ShellError::UnsupportedInput(
                            String::from("Unknown ansi code"),
                            call.positional_nth(0)
                                .expect("Unexpected missing argument")
                                .span,
                        ))
                    }
                }
            }
        } else {
            // This is a record that should look like
            // { fg: "#ff0000" bg: "#00ff00" attr: bli }
            let record = code.as_record()?;
            // create a NuStyle to parse the information into
            let mut nu_style = nu_color_config::NuStyle {
                fg: None,
                bg: None,
                attr: None,
            };
            // Iterate and populate NuStyle with real values
            for (k, v) in record.0.iter().zip(record.1) {
                match k.as_str() {
                    "fg" => nu_style.fg = Some(v.as_string()?),
                    "bg" => nu_style.bg = Some(v.as_string()?),
                    "attr" => nu_style.attr = Some(v.as_string()?),
                    _ => {
                        return Err(ShellError::IncompatibleParametersSingle(
                            format!("problem with key: {}", k),
                            code.span().expect("error with span"),
                        ))
                    }
                }
            }
            // Now create a nu_ansi_term::Style from the NuStyle
            let style = nu_color_config::parse_nustyle(nu_style);
            // Return the prefix string. The prefix is the Ansi String. The suffix would be 0m, reset/stop coloring.
            style.prefix().to_string()
        };

        Ok(Value::string(output, call.head).into_pipeline_data())
    }
}

pub fn str_to_ansi(s: &str) -> Option<String> {
    CODE_MAP.get(s).map(|x| String::from(*x))
}

fn generate_ansi_code_list(
    engine_state: &nu_protocol::engine::EngineState,
    call_span: Span,
    use_ansi_coloring: bool,
) -> Result<nu_protocol::PipelineData, ShellError> {
    return Ok(CODE_LIST
        .iter()
        .enumerate()
        .map(move |(i, ansi_code)| {
            let cols = if use_ansi_coloring {
                vec![
                    "name".into(),
                    "preview".into(),
                    "short name".into(),
                    "code".into(),
                ]
            } else {
                vec!["name".into(), "short name".into(), "code".into()]
            };
            let name: Value = Value::string(String::from(ansi_code.long_name), call_span);
            let short_name = Value::string(ansi_code.short_name.unwrap_or(""), call_span);
            // The first 102 items in the ansi array are colors
            let preview = if i < 375 {
                Value::string(format!("{}NUSHELL\u{1b}[0m", &ansi_code.code), call_span)
            } else {
                Value::string("\u{1b}[0m", call_span)
            };
            let code_string = String::from(&ansi_code.code.replace('\u{1b}', "\\e"));
            let code = Value::string(code_string, call_span);
            let vals = if use_ansi_coloring {
                vec![name, preview, short_name, code]
            } else {
                vec![name, short_name, code]
            };
            Value::Record {
                cols,
                vals,
                span: call_span,
            }
        })
        .into_pipeline_data(engine_state.ctrlc.clone()));
}

fn build_ansi_hashmap(v: &[AnsiCode]) -> HashMap<&str, &str> {
    let mut result = HashMap::new();
    for code in v.iter() {
        let value: &str = &code.code;
        if let Some(sn) = code.short_name {
            result.insert(sn, value);
        }
        result.insert(code.long_name, value);
    }
    result
}

#[cfg(test)]
mod tests {
    use crate::platform::ansi::ansi_::AnsiCommand;

    #[test]
    fn examples_work_as_expected() {
        use crate::test_examples;

        test_examples(AnsiCommand {})
    }
}
