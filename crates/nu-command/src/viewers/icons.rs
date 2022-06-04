use lazy_static::lazy_static;
use nu_protocol::{ShellError, Span};
use std::collections::HashMap;
use std::path::Path;

// Attribution: Thanks exa. Most of this file is taken from around here
// https://github.com/ogham/exa/blob/dbd11d38042284cc890fdd91760c2f93b65e8553/src/output/icons.rs

pub trait FileIcon {
    fn icon_file(&self, file: &Path) -> Option<char>;
}

#[derive(Copy, Clone)]
pub enum Icons {
    Audio,
    Image,
    Video,
}

impl Icons {
    pub fn value(self) -> char {
        match self {
            Self::Audio => '\u{f001}',
            Self::Image => '\u{f1c5}',
            Self::Video => '\u{f03d}',
        }
    }
}

// keeping this for now in case we have to revert to ansi style instead of crossterm style
// Helper function to convert ansi_term style to nu_ansi_term. unfortunately
// this is necessary because ls_colors has a dependency on ansi_term vs nu_ansi_term
// double unfortunately, now we have a dependency on both. we may have to bring
// in ls_colors crate to nushell
// pub fn iconify_style_ansi_to_nu<'a>(style: ansi_term::Style) -> nu_ansi_term::Style {
//     let bg = match style.background {
//         Some(c) => match c {
//             ansi_term::Color::Black => Some(nu_ansi_term::Color::Black),
//             ansi_term::Color::Red => Some(nu_ansi_term::Color::Red),
//             ansi_term::Color::Green => Some(nu_ansi_term::Color::Green),
//             ansi_term::Color::Yellow => Some(nu_ansi_term::Color::Yellow),
//             ansi_term::Color::Blue => Some(nu_ansi_term::Color::Blue),
//             ansi_term::Color::Purple => Some(nu_ansi_term::Color::Purple),
//             ansi_term::Color::Cyan => Some(nu_ansi_term::Color::Cyan),
//             ansi_term::Color::White => Some(nu_ansi_term::Color::White),
//             ansi_term::Color::Fixed(f) => Some(nu_ansi_term::Color::Fixed(f)),
//             ansi_term::Color::RGB(r, g, b) => Some(nu_ansi_term::Color::Rgb(r, g, b)),
//         },
//         None => None,
//     };

//     let fg = match style.foreground {
//         Some(c) => match c {
//             ansi_term::Color::Black => Some(nu_ansi_term::Color::Black),
//             ansi_term::Color::Red => Some(nu_ansi_term::Color::Red),
//             ansi_term::Color::Green => Some(nu_ansi_term::Color::Green),
//             ansi_term::Color::Yellow => Some(nu_ansi_term::Color::Yellow),
//             ansi_term::Color::Blue => Some(nu_ansi_term::Color::Blue),
//             ansi_term::Color::Purple => Some(nu_ansi_term::Color::Purple),
//             ansi_term::Color::Cyan => Some(nu_ansi_term::Color::Cyan),
//             ansi_term::Color::White => Some(nu_ansi_term::Color::White),
//             ansi_term::Color::Fixed(f) => Some(nu_ansi_term::Color::Fixed(f)),
//             ansi_term::Color::RGB(r, g, b) => Some(nu_ansi_term::Color::Rgb(r, g, b)),
//         },
//         None => None,
//     };

//     let nu_style = nu_ansi_term::Style {
//         foreground: fg,
//         background: bg,
//         is_blink: style.is_blink,
//         is_bold: style.is_bold,
//         is_dimmed: style.is_dimmed,
//         is_hidden: style.is_hidden,
//         is_italic: style.is_italic,
//         is_underline: style.is_underline,
//         is_reverse: style.is_reverse,
//         is_strikethrough: style.is_strikethrough,
//     };

//     nu_style
//         .background
//         .or(nu_style.foreground)
//         .map(nu_ansi_term::Style::from)
//         .unwrap_or_default()
// }

lazy_static! {
    static ref MAP_BY_NAME: HashMap<&'static str, char> = {
        let mut m = HashMap::new();
        m.insert(".Trash", '\u{f1f8}'); // 
        m.insert(".atom", '\u{e764}'); // 
        m.insert(".bashprofile", '\u{e615}'); // 
        m.insert(".bashrc", '\u{f489}'); // 
        m.insert(".git", '\u{f1d3}'); // 
        m.insert(".gitattributes", '\u{f1d3}'); // 
        m.insert(".gitconfig", '\u{f1d3}'); // 
        m.insert(".github", '\u{f408}'); // 
        m.insert(".gitignore", '\u{f1d3}'); // 
        m.insert(".gitmodules", '\u{f1d3}'); // 
        m.insert(".rvm", '\u{e21e}'); // 
        m.insert(".vimrc", '\u{e62b}'); // 
        m.insert(".vscode", '\u{e70c}'); // 
        m.insert(".zshrc", '\u{f489}'); // 
        m.insert("Cargo.lock", '\u{e7a8}'); // 
        m.insert("bin", '\u{e5fc}'); // 
        m.insert("config", '\u{e5fc}'); // 
        m.insert("docker-compose.yml", '\u{f308}'); // 
        m.insert("Dockerfile", '\u{f308}'); // 
        m.insert("ds_store", '\u{f179}'); // 
        m.insert("gitignore_global", '\u{f1d3}'); // 
        m.insert("gradle", '\u{e70e}'); // 
        m.insert("gruntfile.coffee", '\u{e611}'); // 
        m.insert("gruntfile.js", '\u{e611}'); // 
        m.insert("gruntfile.ls", '\u{e611}'); // 
        m.insert("gulpfile.coffee", '\u{e610}'); // 
        m.insert("gulpfile.js", '\u{e610}'); // 
        m.insert("gulpfile.ls", '\u{e610}'); // 
        m.insert("hidden", '\u{f023}'); // 
        m.insert("include", '\u{e5fc}'); // 
        m.insert("lib", '\u{f121}'); // 
        m.insert("localized", '\u{f179}'); // 
        m.insert("Makefile", '\u{e779}'); // 
        m.insert("node_modules", '\u{e718}'); // 
        m.insert("npmignore", '\u{e71e}'); // 
        m.insert("rubydoc", '\u{e73b}'); // 
        m.insert("yarn.lock", '\u{e718}'); // 

        m
    };
}

pub fn icon_for_file(file_path: &Path, span: Span) -> Result<char, ShellError> {
    let extensions = Box::new(FileExtensions);
    let fp = format!("{}", file_path.display());

    if let Some(icon) = MAP_BY_NAME.get(&fp[..]) {
        Ok(*icon)
    } else if file_path.is_dir() {
        let str = file_path
            .file_name()
            .ok_or_else(|| {
                ShellError::GenericError(
                    "File name error".into(),
                    "Unable to get file name".into(),
                    Some(span),
                    None,
                    Vec::new(),
                )
            })?
            .to_str()
            .ok_or_else(|| {
                ShellError::GenericError(
                    "Unable to get str error".into(),
                    "Unable to convert to str file name".into(),
                    Some(span),
                    None,
                    Vec::new(),
                )
            })?;
        Ok(match str {
            "bin" => '\u{e5fc}',   // 
            ".git" => '\u{f1d3}',  // 
            ".idea" => '\u{e7b5}', // 
            _ => '\u{f115}',       // 
        })
    } else if let Some(icon) = extensions.icon_file(file_path) {
        Ok(icon)
    } else if let Some(ext) = file_path.extension().as_ref() {
        let str = ext.to_str().ok_or_else(|| {
            ShellError::GenericError(
                "Unable to get str error".into(),
                "Unable to convert to str file name".into(),
                Some(span),
                None,
                Vec::new(),
            )
        })?;
        Ok(match str {
            "ai" => '\u{e7b4}',             // 
            "android" => '\u{e70e}',        // 
            "apk" => '\u{e70e}',            // 
            "apple" => '\u{f179}',          // 
            "avi" => '\u{f03d}',            // 
            "avro" => '\u{e60b}',           // 
            "awk" => '\u{f489}',            // 
            "bash" => '\u{f489}',           // 
            "bash_history" => '\u{f489}',   // 
            "bash_profile" => '\u{f489}',   // 
            "bashrc" => '\u{f489}',         // 
            "bat" => '\u{f17a}',            // 
            "bmp" => '\u{f1c5}',            // 
            "bz" => '\u{f410}',             // 
            "bz2" => '\u{f410}',            // 
            "c" => '\u{e61e}',              // 
            "c++" => '\u{e61d}',            // 
            "cab" => '\u{e70f}',            // 
            "cc" => '\u{e61d}',             // 
            "cfg" => '\u{e615}',            // 
            "class" => '\u{e256}',          // 
            "clj" => '\u{e768}',            // 
            "cljs" => '\u{e76a}',           // 
            "cls" => '\u{e600}',            // 
            "cmd" => '\u{e70f}',            // 
            "coffee" => '\u{f0f4}',         // 
            "conf" => '\u{e615}',           // 
            "cp" => '\u{e61d}',             // 
            "cpp" => '\u{e61d}',            // 
            "cs" => '\u{f81a}',             // 
            "csh" => '\u{f489}',            // 
            "cshtml" => '\u{f1fa}',         // 
            "csproj" => '\u{f81a}',         // 
            "css" => '\u{e749}',            // 
            "csv" => '\u{f1c3}',            // 
            "csx" => '\u{f81a}',            // 
            "cxx" => '\u{e61d}',            // 
            "d" => '\u{e7af}',              // 
            "dart" => '\u{e798}',           // 
            "db" => '\u{f1c0}',             // 
            "deb" => '\u{e77d}',            // 
            "diff" => '\u{f440}',           // 
            "djvu" => '\u{f02d}',           // 
            "dll" => '\u{e70f}',            // 
            "doc" => '\u{f1c2}',            // 
            "docx" => '\u{f1c2}',           // 
            "ds_store" => '\u{f179}',       // 
            "DS_store" => '\u{f179}',       // 
            "dump" => '\u{f1c0}',           // 
            "ebook" => '\u{e28b}',          // 
            "editorconfig" => '\u{e615}',   // 
            "ejs" => '\u{e618}',            // 
            "elm" => '\u{e62c}',            // 
            "env" => '\u{f462}',            // 
            "eot" => '\u{f031}',            // 
            "epub" => '\u{e28a}',           // 
            "erb" => '\u{e73b}',            // 
            "erl" => '\u{e7b1}',            // 
            "ex" => '\u{e62d}',             // 
            "exe" => '\u{f17a}',            // 
            "exs" => '\u{e62d}',            // 
            "fish" => '\u{f489}',           // 
            "flac" => '\u{f001}',           // 
            "flv" => '\u{f03d}',            // 
            "font" => '\u{f031}',           // 
            "gdoc" => '\u{f1c2}',           // 
            "gem" => '\u{e21e}',            // 
            "gemfile" => '\u{e21e}',        // 
            "gemspec" => '\u{e21e}',        // 
            "gform" => '\u{f298}',          // 
            "gif" => '\u{f1c5}',            // 
            "git" => '\u{f1d3}',            // 
            "gitattributes" => '\u{f1d3}',  // 
            "gitignore" => '\u{f1d3}',      // 
            "gitmodules" => '\u{f1d3}',     // 
            "go" => '\u{e626}',             // 
            "gradle" => '\u{e70e}',         // 
            "groovy" => '\u{e775}',         // 
            "gsheet" => '\u{f1c3}',         // 
            "gslides" => '\u{f1c4}',        // 
            "guardfile" => '\u{e21e}',      // 
            "gz" => '\u{f410}',             // 
            "h" => '\u{f0fd}',              // 
            "hbs" => '\u{e60f}',            // 
            "hpp" => '\u{f0fd}',            // 
            "hs" => '\u{e777}',             // 
            "htm" => '\u{f13b}',            // 
            "html" => '\u{f13b}',           // 
            "hxx" => '\u{f0fd}',            // 
            "ico" => '\u{f1c5}',            // 
            "image" => '\u{f1c5}',          // 
            "iml" => '\u{e7b5}',            // 
            "ini" => '\u{f17a}',            // 
            "ipynb" => '\u{e606}',          // 
            "iso" => '\u{e271}',            // 
            "jad" => '\u{e256}',            // 
            "jar" => '\u{e204}',            // 
            "java" => '\u{e204}',           // 
            "jpeg" => '\u{f1c5}',           // 
            "jpg" => '\u{f1c5}',            // 
            "js" => '\u{e74e}',             // 
            "json" => '\u{e60b}',           // 
            "jsx" => '\u{e7ba}',            // 
            "ksh" => '\u{f489}',            // 
            "latex" => '\u{e600}',          // 
            "less" => '\u{e758}',           // 
            "lhs" => '\u{e777}',            // 
            "license" => '\u{f718}',        // 
            "localized" => '\u{f179}',      // 
            "lock" => '\u{f023}',           // 
            "log" => '\u{f18d}',            // 
            "lua" => '\u{e620}',            // 
            "lz" => '\u{f410}',             // 
            "lzh" => '\u{f410}',            // 
            "lzma" => '\u{f410}',           // 
            "lzo" => '\u{f410}',            // 
            "m" => '\u{e61e}',              // 
            "mm" => '\u{e61d}',             // 
            "m4a" => '\u{f001}',            // 
            "markdown" => '\u{f48a}',       // 
            "md" => '\u{f48a}',             // 
            "mjs" => '\u{e74e}',            // 
            "mkd" => '\u{f48a}',            // 
            "mkv" => '\u{f03d}',            // 
            "mobi" => '\u{e28b}',           // 
            "mov" => '\u{f03d}',            // 
            "mp3" => '\u{f001}',            // 
            "mp4" => '\u{f03d}',            // 
            "msi" => '\u{e70f}',            // 
            "mustache" => '\u{e60f}',       // 
            "nix" => '\u{f313}',            // 
            "node" => '\u{f898}',           // 
            "npmignore" => '\u{e71e}',      // 
            "odp" => '\u{f1c4}',            // 
            "ods" => '\u{f1c3}',            // 
            "odt" => '\u{f1c2}',            // 
            "ogg" => '\u{f001}',            // 
            "ogv" => '\u{f03d}',            // 
            "otf" => '\u{f031}',            // 
            "patch" => '\u{f440}',          // 
            "pdf" => '\u{f1c1}',            // 
            "php" => '\u{e73d}',            // 
            "pl" => '\u{e769}',             // 
            "png" => '\u{f1c5}',            // 
            "ppt" => '\u{f1c4}',            // 
            "pptx" => '\u{f1c4}',           // 
            "procfile" => '\u{e21e}',       // 
            "properties" => '\u{e60b}',     // 
            "ps1" => '\u{f489}',            // 
            "psd" => '\u{e7b8}',            // 
            "pxm" => '\u{f1c5}',            // 
            "py" => '\u{e606}',             // 
            "pyc" => '\u{e606}',            // 
            "r" => '\u{f25d}',              // 
            "rakefile" => '\u{e21e}',       // 
            "rar" => '\u{f410}',            // 
            "razor" => '\u{f1fa}',          // 
            "rb" => '\u{e21e}',             // 
            "rdata" => '\u{f25d}',          // 
            "rdb" => '\u{e76d}',            // 
            "rdoc" => '\u{f48a}',           // 
            "rds" => '\u{f25d}',            // 
            "readme" => '\u{f48a}',         // 
            "rlib" => '\u{e7a8}',           // 
            "rmd" => '\u{f48a}',            // 
            "rpm" => '\u{e7bb}',            // 
            "rs" => '\u{e7a8}',             // 
            "rspec" => '\u{e21e}',          // 
            "rspec_parallel" => '\u{e21e}', // 
            "rspec_status" => '\u{e21e}',   // 
            "rss" => '\u{f09e}',            // 
            "rtf" => '\u{f718}',            // 
            "ru" => '\u{e21e}',             // 
            "rubydoc" => '\u{e73b}',        // 
            "sass" => '\u{e603}',           // 
            "scala" => '\u{e737}',          // 
            "scss" => '\u{e749}',           // 
            "sh" => '\u{f489}',             // 
            "shell" => '\u{f489}',          // 
            "slim" => '\u{e73b}',           // 
            "sln" => '\u{e70c}',            // 
            "so" => '\u{f17c}',             // 
            "sql" => '\u{f1c0}',            // 
            "sqlite3" => '\u{e7c4}',        // 
            "styl" => '\u{e600}',           // 
            "stylus" => '\u{e600}',         // 
            "svg" => '\u{f1c5}',            // 
            "swift" => '\u{e755}',          // 
            "tar" => '\u{f410}',            // 
            "taz" => '\u{f410}',            // 
            "tbz" => '\u{f410}',            // 
            "tbz2" => '\u{f410}',           // 
            "tex" => '\u{e600}',            // 
            "tiff" => '\u{f1c5}',           // 
            "toml" => '\u{e615}',           // 
            "ts" => '\u{e628}',             // 
            "tsv" => '\u{f1c3}',            // 
            "tsx" => '\u{e7ba}',            // 
            "ttf" => '\u{f031}',            // 
            "twig" => '\u{e61c}',           // 
            "txt" => '\u{f15c}',            // 
            "tz" => '\u{f410}',             // 
            "tzo" => '\u{f410}',            // 
            "video" => '\u{f03d}',          // 
            "vim" => '\u{e62b}',            // 
            "vue" => '\u{fd42}',            // ﵂
            "war" => '\u{e256}',            // 
            "wav" => '\u{f001}',            // 
            "webm" => '\u{f03d}',           // 
            "webp" => '\u{f1c5}',           // 
            "windows" => '\u{f17a}',        // 
            "woff" => '\u{f031}',           // 
            "woff2" => '\u{f031}',          // 
            "xhtml" => '\u{f13b}',          // 
            "xls" => '\u{f1c3}',            // 
            "xlsx" => '\u{f1c3}',           // 
            "xml" => '\u{fabf}',            // 謹
            "xul" => '\u{fabf}',            // 謹
            "xz" => '\u{f410}',             // 
            "yaml" => '\u{f481}',           // 
            "yml" => '\u{f481}',            // 
            "zip" => '\u{f410}',            // 
            "zsh" => '\u{f489}',            // 
            "zsh-theme" => '\u{f489}',      // 
            "zshrc" => '\u{f489}',          // 
            _ => '\u{f15b}',                // 
        })
    } else {
        Ok('\u{f016}')
    }
}

/// Whether this file’s extension is any of the strings that get passed in.
///
/// This will always return `false` if the file has no extension.
pub fn extension_is_one_of(path: &Path, choices: &[&str]) -> bool {
    match path.extension() {
        Some(os_ext) => match os_ext.to_str() {
            Some(ext) => choices.contains(&ext),
            None => false,
        },
        None => false,
    }
}

/// Whether this file’s name, including extension, is any of the strings
/// that get passed in.
// pub fn name_is_one_of(name: &str, choices: &[&str]) -> bool {
//     choices.contains(&&name[..])
// }

#[derive(Debug, Default, PartialEq, Eq)]
pub struct FileExtensions;

// TODO: We may want to re-add these FileExtensions impl fns back. I have disabled
// it now because it's hard coding colors which kind of defeats the LS_COLORS
// functionality. We may want to enable and augment at some point.

impl FileExtensions {
    //     /// An “immediate” file is something that can be run or activated somehow
    //     /// in order to kick off the build of a project. It’s usually only present
    //     /// in directories full of source code.
    //     #[allow(clippy::case_sensitive_file_extension_comparisons)]
    //     #[allow(dead_code)]
    //     fn is_immediate(&self, file_path: &Path) -> bool {
    //         file_path
    //             .file_name()
    //             .unwrap()
    //             .to_str()
    //             .unwrap()
    //             .to_lowercase()
    //             .starts_with("readme")
    //             || file_path
    //                 .file_name()
    //                 .unwrap()
    //                 .to_str()
    //                 .unwrap()
    //                 .ends_with(".ninja")
    //             || name_is_one_of(
    //                 file_path.file_name().unwrap().to_str().unwrap(),
    //                 &[
    //                     "Makefile",
    //                     "Cargo.toml",
    //                     "SConstruct",
    //                     "CMakeLists.txt",
    //                     "build.gradle",
    //                     "pom.xml",
    //                     "Rakefile",
    //                     "package.json",
    //                     "Gruntfile.js",
    //                     "Gruntfile.coffee",
    //                     "BUILD",
    //                     "BUILD.bazel",
    //                     "WORKSPACE",
    //                     "build.xml",
    //                     "Podfile",
    //                     "webpack.config.js",
    //                     "meson.build",
    //                     "composer.json",
    //                     "RoboFile.php",
    //                     "PKGBUILD",
    //                     "Justfile",
    //                     "Procfile",
    //                     "Dockerfile",
    //                     "Containerfile",
    //                     "Vagrantfile",
    //                     "Brewfile",
    //                     "Gemfile",
    //                     "Pipfile",
    //                     "build.sbt",
    //                     "mix.exs",
    //                     "bsconfig.json",
    //                     "tsconfig.json",
    //                 ],
    //             )
    //     }

    fn is_image(&self, file: &Path) -> bool {
        extension_is_one_of(
            file,
            &[
                "png", "jfi", "jfif", "jif", "jpe", "jpeg", "jpg", "gif", "bmp", "tiff", "tif",
                "ppm", "pgm", "pbm", "pnm", "webp", "raw", "arw", "svg", "stl", "eps", "dvi", "ps",
                "cbr", "jpf", "cbz", "xpm", "ico", "cr2", "orf", "nef", "heif", "avif", "jxl",
            ],
        )
    }

    fn is_video(&self, file: &Path) -> bool {
        extension_is_one_of(
            file,
            &[
                "avi", "flv", "m2v", "m4v", "mkv", "mov", "mp4", "mpeg", "mpg", "ogm", "ogv",
                "vob", "wmv", "webm", "m2ts", "heic",
            ],
        )
    }

    fn is_music(&self, file: &Path) -> bool {
        extension_is_one_of(file, &["aac", "m4a", "mp3", "ogg", "wma", "mka", "opus"])
    }

    // Lossless music, rather than any other kind of data...
    fn is_lossless(&self, file: &Path) -> bool {
        extension_is_one_of(file, &["alac", "ape", "flac", "wav"])
    }

    //     #[allow(dead_code)]
    //     fn is_crypto(&self, file: &Path) -> bool {
    //         extension_is_one_of(
    //             file,
    //             &["asc", "enc", "gpg", "pgp", "sig", "signature", "pfx", "p12"],
    //         )
    //     }

    //     #[allow(dead_code)]
    //     fn is_document(&self, file: &Path) -> bool {
    //         extension_is_one_of(
    //             file,
    //             &[
    //                 "djvu", "doc", "docx", "dvi", "eml", "eps", "fotd", "key", "keynote", "numbers",
    //                 "odp", "odt", "pages", "pdf", "ppt", "pptx", "rtf", "xls", "xlsx",
    //             ],
    //         )
    //     }

    //     #[allow(dead_code)]
    //     fn is_compressed(&self, file: &Path) -> bool {
    //         extension_is_one_of(
    //             file,
    //             &[
    //                 "zip", "tar", "Z", "z", "gz", "bz2", "a", "ar", "7z", "iso", "dmg", "tc", "rar",
    //                 "par", "tgz", "xz", "txz", "lz", "tlz", "lzma", "deb", "rpm", "zst", "lz4",
    //             ],
    //         )
    //     }

    //     #[allow(dead_code)]
    //     fn is_temp(&self, file: &Path) -> bool {
    //         file.file_name().unwrap().to_str().unwrap().ends_with('~')
    //             || (file.file_name().unwrap().to_str().unwrap().starts_with('#')
    //                 && file.file_name().unwrap().to_str().unwrap().ends_with('#'))
    //             || extension_is_one_of(file, &["tmp", "swp", "swo", "swn", "bak", "bkp", "bk"])
    //     }

    //     #[allow(dead_code)]
    //     fn is_compiled(&self, file: &Path) -> bool {
    //         if extension_is_one_of(file, &["class", "elc", "hi", "o", "pyc", "zwc", "ko"]) {
    //             true
    //         // } else if let Some(dir) = file.parent() {
    //         //     file.get_source_files()
    //         //         .iter()
    //         // .any(|path| dir.contains(path))
    //         } else {
    //             false
    //         }
    //     }
    // }

    // impl FileColours for FileExtensions {
    //     fn colour_file(&self, file: &Path) -> Option<Style> {
    //         use ansi_term::Colour::*;

    //         Some(match file {
    //             f if self.is_temp(f)        => Fixed(244).normal(),
    //             f if self.is_immediate(f)   => Yellow.bold().underline(),
    //             f if self.is_image(f)       => Fixed(133).normal(),
    //             f if self.is_video(f)       => Fixed(135).normal(),
    //             f if self.is_music(f)       => Fixed(92).normal(),
    //             f if self.is_lossless(f)    => Fixed(93).normal(),
    //             f if self.is_crypto(f)      => Fixed(109).normal(),
    //             f if self.is_document(f)    => Fixed(105).normal(),
    //             f if self.is_compressed(f)  => Red.normal(),
    //             f if self.is_compiled(f)    => Fixed(137).normal(),
    //             _                           => return None,
    //         })
    //     }
}

impl FileIcon for FileExtensions {
    fn icon_file(&self, file: &Path) -> Option<char> {
        if self.is_music(file) || self.is_lossless(file) {
            Some(Icons::Audio.value())
        } else if self.is_image(file) {
            Some(Icons::Image.value())
        } else if self.is_video(file) {
            Some(Icons::Video.value())
        } else {
            None
        }
    }
}
