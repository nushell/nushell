use serde::Deserialize;
use std::{
    env,
    fs::{self, File},
    io::BufReader,
    path::PathBuf,
    sync::LazyLock,
};

fn main() {
    println!("cargo::rerun-if-changed=build.rs");

    prepare_html_themes();
}

static OUT_DIR: LazyLock<PathBuf> =
    LazyLock::new(|| PathBuf::from(env::var_os("OUT_DIR").expect("set by cargo")));

fn prepare_html_themes() {
    #[path = "src/extra/formats/to/html/theme.rs"]
    mod theme;

    #[derive(Deserialize, Debug)]
    pub struct HtmlThemes {
        pub themes: Vec<theme::HtmlTheme>,
    }

    // 228 themes come from
    // https://github.com/mbadolato/iTerm2-Color-Schemes/tree/master/windowsterminal
    println!("cargo::rerun-if-changed=assets/228_themes.json");

    let themes = File::open("assets/228_themes.json").unwrap();
    let themes = BufReader::new(themes);
    let themes: HtmlThemes = serde_json::from_reader(themes).unwrap();

    let themes = themes
        .themes
        .into_iter()
        .map(
            |theme::HtmlTheme {
                 name,
                 black,
                 red,
                 green,
                 yellow,
                 blue,
                 purple,
                 cyan,
                 white,
                 bright_black,
                 bright_red,
                 bright_green,
                 bright_yellow,
                 bright_blue,
                 bright_purple,
                 bright_cyan,
                 bright_white,
                 background,
                 foreground,
             }| {
                quote::quote! {
                    HtmlTheme {
                        name: ::std::borrow::Cow::Borrowed(#name),
                        black: ::std::borrow::Cow::Borrowed(#black),
                        red: ::std::borrow::Cow::Borrowed(#red),
                        green: ::std::borrow::Cow::Borrowed(#green),
                        yellow: ::std::borrow::Cow::Borrowed(#yellow),
                        blue: ::std::borrow::Cow::Borrowed(#blue),
                        purple: ::std::borrow::Cow::Borrowed(#purple),
                        cyan: ::std::borrow::Cow::Borrowed(#cyan),
                        white: ::std::borrow::Cow::Borrowed(#white),
                        bright_black: ::std::borrow::Cow::Borrowed(#bright_black),
                        bright_red: ::std::borrow::Cow::Borrowed(#bright_red),
                        bright_green: ::std::borrow::Cow::Borrowed(#bright_green),
                        bright_yellow: ::std::borrow::Cow::Borrowed(#bright_yellow),
                        bright_blue: ::std::borrow::Cow::Borrowed(#bright_blue),
                        bright_purple: ::std::borrow::Cow::Borrowed(#bright_purple),
                        bright_cyan: ::std::borrow::Cow::Borrowed(#bright_cyan),
                        bright_white: ::std::borrow::Cow::Borrowed(#bright_white),
                        background: ::std::borrow::Cow::Borrowed(#background),
                        foreground: ::std::borrow::Cow::Borrowed(#foreground),
                    }
                }
            },
        )
        .collect::<Vec<_>>();

    let code = quote::quote! {
        impl HtmlTheme {
            pub fn list() -> &'static [HtmlTheme] {
                &[#(#themes),*]
            }
        }
    };

    let path = OUT_DIR.join("html_theme_list.rs");
    fs::write(path, code.to_string()).unwrap();
}
