use ansi_term::{Color, Style};
use lscolors::{LsColors, Style as LsStyle};
use nu_protocol::{hir::Number, value::StrExt, Primitive, UntaggedValue, Value};
use nu_source::Tag;
use nu_table::{Alignment, TextStyle};
use std::collections::HashMap;

static DEFAULT_COLORS: &str = "pi=0;38;2;0;0;0;48;2;102;217;239:so=0;38;2;0;0;0;48;2;249;38;114:*~=0;38;2;122;112;112:ex=1;38;2;249;38;114:ln=0;38;2;249;38;114:fi=0:or=0;38;2;0;0;0;48;2;255;74;68:di=0;38;2;102;217;239:no=0:mi=0;38;2;0;0;0;48;2;255;74;68:*.r=0;38;2;0;255;135:*.o=0;38;2;122;112;112:*.h=0;38;2;0;255;135:*.p=0;38;2;0;255;135:*.t=0;38;2;0;255;135:*.a=1;38;2;249;38;114:*.z=4;38;2;249;38;114:*.m=0;38;2;0;255;135:*.c=0;38;2;0;255;135:*.d=0;38;2;0;255;135:*.pl=0;38;2;0;255;135:*.pm=0;38;2;0;255;135:*.pp=0;38;2;0;255;135:*.ko=1;38;2;249;38;114:*.ui=0;38;2;166;226;46:*.ps=0;38;2;230;219;116:*.di=0;38;2;0;255;135:*.sh=0;38;2;0;255;135:*.rb=0;38;2;0;255;135:*.cc=0;38;2;0;255;135:*.cr=0;38;2;0;255;135:*.hi=0;38;2;122;112;112:*.xz=4;38;2;249;38;114:*.go=0;38;2;0;255;135:*.bz=4;38;2;249;38;114:*.7z=4;38;2;249;38;114:*.rm=0;38;2;253;151;31:*.cp=0;38;2;0;255;135:*.hh=0;38;2;0;255;135:*.cs=0;38;2;0;255;135:*.el=0;38;2;0;255;135:*.kt=0;38;2;0;255;135:*.py=0;38;2;0;255;135:*.mn=0;38;2;0;255;135:*.hs=0;38;2;0;255;135:*.la=0;38;2;122;112;112:*.vb=0;38;2;0;255;135:*.md=0;38;2;226;209;57:*.rs=0;38;2;0;255;135:*.ml=0;38;2;0;255;135:*.so=1;38;2;249;38;114:*.ts=0;38;2;0;255;135:*.as=0;38;2;0;255;135:*.gz=4;38;2;249;38;114:*.ex=0;38;2;0;255;135:*.jl=0;38;2;0;255;135:*css=0;38;2;0;255;135:*.gv=0;38;2;0;255;135:*.js=0;38;2;0;255;135:*.nb=0;38;2;0;255;135:*.fs=0;38;2;0;255;135:*.lo=0;38;2;122;112;112:*.tml=0;38;2;166;226;46:*.pro=0;38;2;166;226;46:*.pas=0;38;2;0;255;135:*.bin=4;38;2;249;38;114:*.xcf=0;38;2;253;151;31:*.ini=0;38;2;166;226;46:*.fsi=0;38;2;0;255;135:*.ics=0;38;2;230;219;116:*.sbt=0;38;2;0;255;135:*.tar=4;38;2;249;38;114:*.deb=4;38;2;249;38;114:*.cgi=0;38;2;0;255;135:*.xmp=0;38;2;166;226;46:*.hxx=0;38;2;0;255;135:*.cfg=0;38;2;166;226;46:*.bag=4;38;2;249;38;114:*.ppt=0;38;2;230;219;116:*.asa=0;38;2;0;255;135:*.xls=0;38;2;230;219;116:*.htm=0;38;2;226;209;57:*.h++=0;38;2;0;255;135:*.hpp=0;38;2;0;255;135:*.tex=0;38;2;0;255;135:*.tmp=0;38;2;122;112;112:*.erl=0;38;2;0;255;135:*.cxx=0;38;2;0;255;135:*.inl=0;38;2;0;255;135:*.elm=0;38;2;0;255;135:*.kts=0;38;2;0;255;135:*.bz2=4;38;2;249;38;114:*.arj=4;38;2;249;38;114:*.dmg=4;38;2;249;38;114:*.vcd=4;38;2;249;38;114:*.ipp=0;38;2;0;255;135:*TODO=1:*.m4v=0;38;2;253;151;31:*.git=0;38;2;122;112;112:*.pod=0;38;2;0;255;135:*.svg=0;38;2;253;151;31:*.log=0;38;2;122;112;112:*.pgm=0;38;2;253;151;31:*.vim=0;38;2;0;255;135:*.bib=0;38;2;166;226;46:*.rpm=4;38;2;249;38;114:*.mpg=0;38;2;253;151;31:*.dpr=0;38;2;0;255;135:*.aux=0;38;2;122;112;112:*.tsx=0;38;2;0;255;135:*.odt=0;38;2;230;219;116:*.mli=0;38;2;0;255;135:*.ps1=0;38;2;0;255;135:*.cpp=0;38;2;0;255;135:*.flv=0;38;2;253;151;31:*.fsx=0;38;2;0;255;135:*.tif=0;38;2;253;151;31:*.blg=0;38;2;122;112;112:*.sty=0;38;2;122;112;112:*.bak=0;38;2;122;112;112:*.zip=4;38;2;249;38;114:*.sxw=0;38;2;230;219;116:*.clj=0;38;2;0;255;135:*.mkv=0;38;2;253;151;31:*.doc=0;38;2;230;219;116:*.dox=0;38;2;166;226;46:*.swf=0;38;2;253;151;31:*.rst=0;38;2;226;209;57:*.png=0;38;2;253;151;31:*.pid=0;38;2;122;112;112:*.nix=0;38;2;166;226;46:*.aif=0;38;2;253;151;31:*.ogg=0;38;2;253;151;31:*.tgz=4;38;2;249;38;114:*.otf=0;38;2;253;151;31:*.img=4;38;2;249;38;114:*.txt=0;38;2;226;209;57:*.epp=0;38;2;0;255;135:*.jpg=0;38;2;253;151;31:*.c++=0;38;2;0;255;135:*.ppm=0;38;2;253;151;31:*.dll=1;38;2;249;38;114:*.tcl=0;38;2;0;255;135:*.sxi=0;38;2;230;219;116:*.bat=1;38;2;249;38;114:*.mid=0;38;2;253;151;31:*.vob=0;38;2;253;151;31:*.csx=0;38;2;0;255;135:*.idx=0;38;2;122;112;112:*.wma=0;38;2;253;151;31:*hgrc=0;38;2;166;226;46:*.fls=0;38;2;122;112;112:*.lua=0;38;2;0;255;135:*.pkg=4;38;2;249;38;114:*.csv=0;38;2;226;209;57:*.wmv=0;38;2;253;151;31:*.fon=0;38;2;253;151;31:*.avi=0;38;2;253;151;31:*.pps=0;38;2;230;219;116:*.swp=0;38;2;122;112;112:*.iso=4;38;2;249;38;114:*.bcf=0;38;2;122;112;112:*.exe=1;38;2;249;38;114:*.bmp=0;38;2;253;151;31:*.pyc=0;38;2;122;112;112:*.apk=4;38;2;249;38;114:*.ttf=0;38;2;253;151;31:*.yml=0;38;2;166;226;46:*.rar=4;38;2;249;38;114:*.zsh=0;38;2;0;255;135:*.xml=0;38;2;226;209;57:*.htc=0;38;2;0;255;135:*.kex=0;38;2;230;219;116:*.com=1;38;2;249;38;114:*.fnt=0;38;2;253;151;31:*.xlr=0;38;2;230;219;116:*.ods=0;38;2;230;219;116:*.ltx=0;38;2;0;255;135:*.bbl=0;38;2;122;112;112:*.odp=0;38;2;230;219;116:*.ilg=0;38;2;122;112;112:*.exs=0;38;2;0;255;135:*.wav=0;38;2;253;151;31:*.bst=0;38;2;166;226;46:*.pbm=0;38;2;253;151;31:*.sql=0;38;2;0;255;135:*.dot=0;38;2;0;255;135:*.awk=0;38;2;0;255;135:*.tbz=4;38;2;249;38;114:*.toc=0;38;2;122;112;112:*.out=0;38;2;122;112;112:*.mp4=0;38;2;253;151;31:*.ind=0;38;2;122;112;112:*.bsh=0;38;2;0;255;135:*.jar=4;38;2;249;38;114:*.mov=0;38;2;253;151;31:*.ico=0;38;2;253;151;31:*.gvy=0;38;2;0;255;135:*.gif=0;38;2;253;151;31:*.rtf=0;38;2;230;219;116:*.php=0;38;2;0;255;135:*.mp3=0;38;2;253;151;31:*.pdf=0;38;2;230;219;116:*.toml=0;38;2;166;226;46:*.flac=0;38;2;253;151;31:*.conf=0;38;2;166;226;46:*.mpeg=0;38;2;253;151;31:*.hgrc=0;38;2;166;226;46:*.h264=0;38;2;253;151;31:*.yaml=0;38;2;166;226;46:*.json=0;38;2;166;226;46:*.tbz2=4;38;2;249;38;114:*.lock=0;38;2;122;112;112:*.diff=0;38;2;0;255;135:*.xlsx=0;38;2;230;219;116:*.rlib=0;38;2;122;112;112:*.java=0;38;2;0;255;135:*.fish=0;38;2;0;255;135:*.docx=0;38;2;230;219;116:*.html=0;38;2;226;209;57:*.make=0;38;2;166;226;46:*.less=0;38;2;0;255;135:*.pptx=0;38;2;230;219;116:*.epub=0;38;2;230;219;116:*.psm1=0;38;2;0;255;135:*.jpeg=0;38;2;253;151;31:*.lisp=0;38;2;0;255;135:*.orig=0;38;2;122;112;112:*.dart=0;38;2;0;255;135:*.bash=0;38;2;0;255;135:*.purs=0;38;2;0;255;135:*.psd1=0;38;2;0;255;135:*.shtml=0;38;2;226;209;57:*.class=0;38;2;122;112;112:*.cmake=0;38;2;166;226;46:*.cabal=0;38;2;0;255;135:*.scala=0;38;2;0;255;135:*.ipynb=0;38;2;0;255;135:*passwd=0;38;2;166;226;46:*README=0;38;2;0;0;0;48;2;230;219;116:*.swift=0;38;2;0;255;135:*.dyn_o=0;38;2;122;112;112:*shadow=0;38;2;166;226;46:*.patch=0;38;2;0;255;135:*.toast=4;38;2;249;38;114:*.xhtml=0;38;2;226;209;57:*.cache=0;38;2;122;112;112:*.mdown=0;38;2;226;209;57:*COPYING=0;38;2;182;182;182:*TODO.md=1:*.config=0;38;2;166;226;46:*.dyn_hi=0;38;2;122;112;112:*.ignore=0;38;2;166;226;46:*INSTALL=0;38;2;0;0;0;48;2;230;219;116:*LICENSE=0;38;2;182;182;182:*.gradle=0;38;2;0;255;135:*.groovy=0;38;2;0;255;135:*.matlab=0;38;2;0;255;135:*.flake8=0;38;2;166;226;46:*.gemspec=0;38;2;166;226;46:*setup.py=0;38;2;166;226;46:*Makefile=0;38;2;166;226;46:*Doxyfile=0;38;2;166;226;46:*.desktop=0;38;2;166;226;46:*TODO.txt=1:*.kdevelop=0;38;2;166;226;46:*COPYRIGHT=0;38;2;182;182;182:*.cmake.in=0;38;2;166;226;46:*.rgignore=0;38;2;166;226;46:*README.md=0;38;2;0;0;0;48;2;230;219;116:*.markdown=0;38;2;226;209;57:*configure=0;38;2;166;226;46:*.fdignore=0;38;2;166;226;46:*Dockerfile=0;38;2;166;226;46:*README.txt=0;38;2;0;0;0;48;2;230;219;116:*INSTALL.md=0;38;2;0;0;0;48;2;230;219;116:*.gitignore=0;38;2;166;226;46:*SConscript=0;38;2;166;226;46:*.scons_opt=0;38;2;122;112;112:*SConstruct=0;38;2;166;226;46:*CODEOWNERS=0;38;2;166;226;46:*.gitconfig=0;38;2;166;226;46:*.synctex.gz=0;38;2;122;112;112:*.gitmodules=0;38;2;166;226;46:*Makefile.am=0;38;2;166;226;46:*LICENSE-MIT=0;38;2;182;182;182:*Makefile.in=0;38;2;122;112;112:*MANIFEST.in=0;38;2;166;226;46:*.travis.yml=0;38;2;230;219;116:*CONTRIBUTORS=0;38;2;0;0;0;48;2;230;219;116:*configure.ac=0;38;2;166;226;46:*.applescript=0;38;2;0;255;135:*appveyor.yml=0;38;2;230;219;116:*.fdb_latexmk=0;38;2;122;112;112:*.clang-format=0;38;2;166;226;46:*LICENSE-APACHE=0;38;2;182;182;182:*INSTALL.md.txt=0;38;2;0;0;0;48;2;230;219;116:*CMakeLists.txt=0;38;2;166;226;46:*.gitattributes=0;38;2;166;226;46:*CMakeCache.txt=0;38;2;122;112;112:*CONTRIBUTORS.md=0;38;2;0;0;0;48;2;230;219;116:*CONTRIBUTORS.txt=0;38;2;0;0;0;48;2;230;219;116:*.sconsign.dblite=0;38;2;122;112;112:*requirements.txt=0;38;2;166;226;46:*package-lock.json=0;38;2;122;112;112";

pub fn number(number: impl Into<Number>) -> Primitive {
    let number = number.into();

    match number {
        Number::Int(int) => Primitive::Int(int),
        Number::Decimal(decimal) => Primitive::Decimal(decimal),
    }
}

pub fn lookup_ansi_color_style(s: String) -> Style {
    match s.as_str() {
        "g" | "green" => Color::Green.normal(),
        "gb" | "green_bold" => Color::Green.bold(),
        "gu" | "green_underline" => Color::Green.underline(),
        "gi" | "green_italic" => Color::Green.italic(),
        "gd" | "green_dimmed" => Color::Green.dimmed(),
        "gr" | "green_reverse" => Color::Green.reverse(),
        "gbl" | "green_blink" => Color::Green.blink(),
        "gst" | "green_strike" => Color::Green.strikethrough(),
        "r" | "red" => Color::Red.normal(),
        "rb" | "red_bold" => Color::Red.bold(),
        "ru" | "red_underline" => Color::Red.underline(),
        "ri" | "red_italic" => Color::Red.italic(),
        "rd" | "red_dimmed" => Color::Red.dimmed(),
        "rr" | "red_reverse" => Color::Red.reverse(),
        "rbl" | "red_blink" => Color::Red.blink(),
        "rst" | "red_strike" => Color::Red.strikethrough(),
        "u" | "blue" => Color::Blue.normal(),
        "ub" | "blue_bold" => Color::Blue.bold(),
        "uu" | "blue_underline" => Color::Blue.underline(),
        "ui" | "blue_italic" => Color::Blue.italic(),
        "ud" | "blue_dimmed" => Color::Blue.dimmed(),
        "ur" | "blue_reverse" => Color::Blue.reverse(),
        "ubl" | "blue_blink" => Color::Blue.blink(),
        "ust" | "blue_strike" => Color::Blue.strikethrough(),
        "b" | "black" => Color::Black.normal(),
        "bb" | "black_bold" => Color::Black.bold(),
        "bu" | "black_underline" => Color::Black.underline(),
        "bi" | "black_italic" => Color::Black.italic(),
        "bd" | "black_dimmed" => Color::Black.dimmed(),
        "br" | "black_reverse" => Color::Black.reverse(),
        "bbl" | "black_blink" => Color::Black.blink(),
        "bst" | "black_strike" => Color::Black.strikethrough(),
        "y" | "yellow" => Color::Yellow.normal(),
        "yb" | "yellow_bold" => Color::Yellow.bold(),
        "yu" | "yellow_underline" => Color::Yellow.underline(),
        "yi" | "yellow_italic" => Color::Yellow.italic(),
        "yd" | "yellow_dimmed" => Color::Yellow.dimmed(),
        "yr" | "yellow_reverse" => Color::Yellow.reverse(),
        "ybl" | "yellow_blink" => Color::Yellow.blink(),
        "yst" | "yellow_strike" => Color::Yellow.strikethrough(),
        "p" | "purple" => Color::Purple.normal(),
        "pb" | "purple_bold" => Color::Purple.bold(),
        "pu" | "purple_underline" => Color::Purple.underline(),
        "pi" | "purple_italic" => Color::Purple.italic(),
        "pd" | "purple_dimmed" => Color::Purple.dimmed(),
        "pr" | "purple_reverse" => Color::Purple.reverse(),
        "pbl" | "purple_blink" => Color::Purple.blink(),
        "pst" | "purple_strike" => Color::Purple.strikethrough(),
        "c" | "cyan" => Color::Cyan.normal(),
        "cb" | "cyan_bold" => Color::Cyan.bold(),
        "cu" | "cyan_underline" => Color::Cyan.underline(),
        "ci" | "cyan_italic" => Color::Cyan.italic(),
        "cd" | "cyan_dimmed" => Color::Cyan.dimmed(),
        "cr" | "cyan_reverse" => Color::Cyan.reverse(),
        "cbl" | "cyan_blink" => Color::Cyan.blink(),
        "cst" | "cyan_strike" => Color::Cyan.strikethrough(),
        "w" | "white" => Color::White.normal(),
        "wb" | "white_bold" => Color::White.bold(),
        "wu" | "white_underline" => Color::White.underline(),
        "wi" | "white_italic" => Color::White.italic(),
        "wd" | "white_dimmed" => Color::White.dimmed(),
        "wr" | "white_reverse" => Color::White.reverse(),
        "wbl" | "white_blink" => Color::White.blink(),
        "wst" | "white_strike" => Color::White.strikethrough(),
        "true" => Color::RGB(1, 1, 1).normal(),
        _ => Color::White.normal(),
    }
}

pub fn string_to_lookup_value(str_prim: &str) -> String {
    match str_prim {
        "primitive_int" => "Primitive::Int".to_string(),
        "primitive_decimal" => "Primitive::Decimal".to_string(),
        "primitive_filesize" => "Primitive::Filesize".to_string(),
        "primitive_string" => "Primitive::String".to_string(),
        "primitive_line" => "Primitive::Line".to_string(),
        "primitive_columnpath" => "Primitive::ColumnPath".to_string(),
        "primitive_pattern" => "Primitive::Pattern".to_string(),
        "primitive_boolean" => "Primitive::Boolean".to_string(),
        "primitive_date" => "Primitive::Date".to_string(),
        "primitive_duration" => "Primitive::Duration".to_string(),
        "primitive_range" => "Primitive::Range".to_string(),
        "primitive_path" => "Primitive::Path".to_string(),
        "primitive_binary" => "Primitive::Binary".to_string(),
        "separator_color" => "separator_color".to_string(),
        "header_align" => "header_align".to_string(),
        "header_color" => "header_color".to_string(),
        "header_bold" => "header_bold".to_string(),
        "header_style" => "header_style".to_string(),
        "index_color" => "index_color".to_string(),
        "leading_trailing_space_bg" => "leading_trailing_space_bg".to_string(),
        "use_ls_colors" => "use_ls_colors".to_string(),
        _ => "Primitive::Nothing".to_string(),
    }
}

fn update_hashmap(key: &str, val: &Value, hm: &mut HashMap<String, Style>) {
    if let Ok(var) = val.as_string() {
        let color = lookup_ansi_color_style(var);
        let prim = string_to_lookup_value(&key);
        if let Some(v) = hm.get_mut(&prim) {
            *v = color;
        } else {
            hm.insert(prim, color);
        }
    }
}

pub fn get_color_config() -> HashMap<String, Style> {
    // create the hashmap
    let mut hm: HashMap<String, Style> = HashMap::new();
    // set some defaults
    hm.insert("primitive_int".to_string(), Color::White.normal());
    hm.insert("primitive_decimal".to_string(), Color::White.normal());
    hm.insert("primitive_filesize".to_string(), Color::White.normal());
    hm.insert("primitive_string".to_string(), Color::White.normal());
    hm.insert("primitive_line".to_string(), Color::White.normal());
    hm.insert("primitive_columnpath".to_string(), Color::White.normal());
    hm.insert("primitive_pattern".to_string(), Color::White.normal());
    hm.insert("primitive_boolean".to_string(), Color::White.normal());
    hm.insert("primitive_date".to_string(), Color::White.normal());
    hm.insert("primitive_duration".to_string(), Color::White.normal());
    hm.insert("primitive_range".to_string(), Color::White.normal());
    hm.insert("primitive_path".to_string(), Color::White.normal());
    hm.insert("primitive_binary".to_string(), Color::White.normal());
    hm.insert("separator_color".to_string(), Color::White.normal());
    hm.insert("header_align".to_string(), Color::Green.bold());
    hm.insert("header_color".to_string(), Color::Green.bold());
    hm.insert("header_bold".to_string(), Color::Green.bold());
    hm.insert("header_style".to_string(), Style::default());
    hm.insert("index_color".to_string(), Color::Green.bold());
    hm.insert(
        "leading_trailing_space_bg".to_string(),
        Style::default().on(Color::RGB(128, 128, 128)),
    );
    // FIXME:
    // this is a total hack in order to use this hashmap for boolean values
    // if use_ls_colors = true in the color_config then the style will be RGB(1,1,1)
    // which is an oddball color. So, when we see this color assume it's true
    // otherwise assume it's false. Initialize to RGB(1,2,3).
    hm.insert("use_ls_colors".to_string(), Color::RGB(1, 2, 3).normal());

    // populate hashmap from config values
    if let Ok(config) = crate::config::config(Tag::unknown()) {
        if let Some(primitive_color_vars) = config.get("color_config") {
            for (key, value) in primitive_color_vars.row_entries() {
                match key.as_ref() {
                    "primitive_int" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "primitive_decimal" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "primitive_filesize" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "primitive_string" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "primitive_line" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "primitive_columnpath" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "primitive_pattern" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "primitive_boolean" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "primitive_date" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "primitive_duration" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "primitive_range" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "primitive_path" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "primitive_binary" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "separator_color" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "header_align" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "header_color" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "header_bold" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "header_style" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "index_color" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "leading_trailing_space_bg" => {
                        update_hashmap(&key, &value, &mut hm);
                    }
                    "use_ls_colors" => {
                        // FIXME: This is a hack. If use_ls_colors is set to any valid toml entry
                        // set it to true, which turns the style into RGB(1,1,1) which we can check
                        // later
                        update_hashmap(&key, &"true".to_str_value_create_tag(), &mut hm);
                    }
                    _ => (),
                }
            }
        }
    }

    hm
}

// This function will assign a text style to a primitive, or really any string that's
// in the hashmap. The hashmap actually contains the style to be applied.
pub fn style_primitive(
    primitive: &str,
    color_hm: &HashMap<String, Style>,
    value: &UntaggedValue,
) -> TextStyle {
    let use_ls_colors = color_hm
        .get("use_ls_colors")
        .unwrap_or(&Color::RGB(1, 2, 3).normal())
        .to_owned();

    match primitive {
        "Int" => {
            let style = color_hm.get("Primitive::Int");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Right, *s),
                None => TextStyle::basic_right(),
            }
        }
        "Decimal" => {
            let style = color_hm.get("Primitive::Decimal");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Right, *s),
                None => TextStyle::basic_right(),
            }
        }
        "Filesize" => {
            let style = color_hm.get("Primitive::Filesize");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Right, *s),
                None => TextStyle::basic_right(),
            }
        }
        "String" => {
            let style = color_hm.get("Primitive::String");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_left(),
            }
        }
        "Line" => {
            let style = color_hm.get("Primitive::Line");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_left(),
            }
        }
        "ColumnPath" => {
            let style = color_hm.get("Primitive::ColumnPath");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_left(),
            }
        }
        "Pattern" => {
            let style = color_hm.get("Primitive::Pattern");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_left(),
            }
        }
        "Boolean" => {
            let style = color_hm.get("Primitive::Boolean");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_left(),
            }
        }
        "Date" => {
            let style = color_hm.get("Primitive::Date");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_left(),
            }
        }
        "Duration" => {
            let style = color_hm.get("Primitive::Duration");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_left(),
            }
        }
        "Range" => {
            let style = color_hm.get("Primitive::Range");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_left(),
            }
        }
        "Path" => {
            // FIXME: This is a hack. If use_ls_colors = RGB(1,1,1) that means
            // they set use_ls_colors = true in the config file.
            if use_ls_colors == Color::RGB(1, 1, 1).normal() {
                let file_path = value
                    .clone()
                    .into_untagged_value()
                    .as_path()
                    .expect("problem with file path");
                get_file_style(&file_path)
            } else {
                let style = color_hm.get("Primitive::Path");
                match style {
                    Some(s) => TextStyle::with_style(Alignment::Left, *s),
                    None => TextStyle::basic_left(),
                }
            }
        }
        "Binary" => {
            let style = color_hm.get("Primitive::Binary");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_left(),
            }
        }
        "BeginningOfStream" => {
            let style = color_hm.get("Primitive::BeginningOfStream");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_left(),
            }
        }
        "EndOfStream" => {
            let style = color_hm.get("Primitive::EndOfStream");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_left(),
            }
        }
        "Nothing" => {
            let style = color_hm.get("Primitive::Nothing");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_left(),
            }
        }
        "separator_color" => {
            let style = color_hm.get("separator");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Left, *s),
                None => TextStyle::basic_left(),
            }
        }
        "header_align" => {
            let style = color_hm.get("header_align");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Center, *s),
                None => TextStyle::default_header(),
            }
        }
        "header_color" => {
            let style = color_hm.get("header_color");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Center, *s),
                None => TextStyle::default_header(),
            }
        }
        "header_bold" => {
            let style = color_hm.get("header_bold");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Center, *s),
                None => TextStyle::default_header(),
            }
        }
        "header_style" => {
            let style = color_hm.get("header_style");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Center, *s),
                None => TextStyle::default_header(),
            }
        }
        "index_color" => {
            let style = color_hm.get("index_color");
            match style {
                Some(s) => TextStyle::with_style(Alignment::Right, *s),
                None => TextStyle::new()
                    .alignment(Alignment::Right)
                    .fg(Color::Green)
                    .bold(Some(true)),
            }
        }
        _ => TextStyle::basic_center(),
    }
}

fn get_file_style(name: &std::path::Path) -> TextStyle {
    let default_ls_colors = LsColors::from_string(DEFAULT_COLORS);
    let ls_colors = LsColors::from_env().unwrap_or(default_ls_colors);
    let style = ls_colors.style_for_path(name);
    let ansi_style = style.map(LsStyle::to_ansi_term_style).unwrap_or_default();

    TextStyle::with_style(Alignment::Left, ansi_style)
}
