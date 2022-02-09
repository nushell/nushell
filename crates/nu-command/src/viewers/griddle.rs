use std::borrow::Cow;

// use super::icons::{icon_for_file, iconify_style_ansi_to_nu};
use super::icons::icon_for_file;
use lscolors::{LsColors, Style};
use nu_engine::env_to_string;
use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, PathMember},
    engine::{Command, EngineState, Stack},
    Category, Config, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape,
    Value,
};
use nu_term_grid::grid::{Alignment, Cell, Direction, Filling, Grid, GridOptions};
use terminal_size::{Height, Width};

#[derive(Clone)]
pub struct Griddle;

impl Command for Griddle {
    fn name(&self) -> &str {
        "grid"
    }

    fn usage(&self) -> &str {
        "Renders the output to a textual terminal grid."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("grid")
            .named(
                "width",
                SyntaxShape::Int,
                "number of columns wide",
                Some('w'),
            )
            .switch("color", "draw output with color", Some('c'))
            .named(
                "separator",
                SyntaxShape::String,
                "character to separate grid with",
                Some('s'),
            )
            .category(Category::Viewers)
    }

    fn extra_usage(&self) -> &str {
        r#"grid was built to give a concise gridded layout for ls. however,
it determines what to put in the grid by looking for a column named
'name'. this works great for tables and records but for lists we
need to do something different. such as with '[one two three] | grid'
it creates a fake column called 'name' for these values so that it
prints out the list properly."#
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let width_param: Option<String> = call.get_flag(engine_state, stack, "width")?;
        let color_param: bool = call.has_flag("color");
        let separator_param: Option<String> = call.get_flag(engine_state, stack, "separator")?;
        let config = stack.get_config().unwrap_or_default();
        let env_str = match stack.get_env_var(engine_state, "LS_COLORS") {
            Some(v) => Some(env_to_string("LS_COLORS", v, engine_state, stack, &config)?),
            None => None,
        };
        let use_grid_icons = config.use_grid_icons;

        match input {
            PipelineData::Value(Value::List { vals, .. }, ..) => {
                // dbg!("value::list");
                let data = convert_to_list(vals, &config, call.head);
                if let Some(items) = data {
                    Ok(create_grid_output(
                        items,
                        call,
                        width_param,
                        color_param,
                        separator_param,
                        env_str,
                        use_grid_icons,
                    )?)
                } else {
                    Ok(PipelineData::new(call.head))
                }
            }
            PipelineData::ListStream(stream, ..) => {
                // dbg!("value::stream");
                let data = convert_to_list(stream, &config, call.head);
                if let Some(items) = data {
                    Ok(create_grid_output(
                        items,
                        call,
                        width_param,
                        color_param,
                        separator_param,
                        env_str,
                        use_grid_icons,
                    )?)
                } else {
                    // dbg!(data);
                    Ok(PipelineData::new(call.head))
                }
            }
            PipelineData::Value(Value::Record { cols, vals, .. }, ..) => {
                // dbg!("value::record");
                let mut items = vec![];

                for (i, (c, v)) in cols.into_iter().zip(vals.into_iter()).enumerate() {
                    items.push((i, c, v.into_string(", ", &config)))
                }

                Ok(create_grid_output(
                    items,
                    call,
                    width_param,
                    color_param,
                    separator_param,
                    env_str,
                    use_grid_icons,
                )?)
            }
            x => {
                // dbg!("other value");
                // dbg!(x.get_type());
                Ok(x)
            }
        }
    }
}

/// Removes ANSI escape codes and some ASCII control characters
///
/// Keeps `\n` removes `\r`, `\t` etc.
///
/// If parsing fails silently returns the input string
fn strip_ansi(string: &str) -> Cow<str> {
    // Check if any ascii control character except LF(0x0A = 10) is present,
    // which will be stripped. Includes the primary start of ANSI sequences ESC
    // (0x1B = decimal 27)
    if string.bytes().any(|x| matches!(x, 0..=9 | 11..=31)) {
        if let Ok(stripped) = strip_ansi_escapes::strip(string) {
            if let Ok(new_string) = String::from_utf8(stripped) {
                return Cow::Owned(new_string);
            }
        }
    }
    // Else case includes failures to parse!
    Cow::Borrowed(string)
}

fn create_grid_output(
    items: Vec<(usize, String, String)>,
    call: &Call,
    width_param: Option<String>,
    color_param: bool,
    separator_param: Option<String>,
    env_str: Option<String>,
    use_grid_icons: bool,
) -> Result<PipelineData, ShellError> {
    let ls_colors = match env_str {
        Some(s) => LsColors::from_string(&s),
        None => LsColors::from_string("st=0:di=0;38;5;81:so=0;38;5;16;48;5;203:ln=0;38;5;203:cd=0;38;5;203;48;5;236:ex=1;38;5;203:or=0;38;5;16;48;5;203:fi=0:bd=0;38;5;81;48;5;236:ow=0:mi=0;38;5;16;48;5;203:*~=0;38;5;243:no=0:tw=0:pi=0;38;5;16;48;5;81:*.z=4;38;5;203:*.t=0;38;5;48:*.o=0;38;5;243:*.d=0;38;5;48:*.a=1;38;5;203:*.c=0;38;5;48:*.m=0;38;5;48:*.p=0;38;5;48:*.r=0;38;5;48:*.h=0;38;5;48:*.ml=0;38;5;48:*.ll=0;38;5;48:*.gv=0;38;5;48:*.cp=0;38;5;48:*.xz=4;38;5;203:*.hs=0;38;5;48:*css=0;38;5;48:*.ui=0;38;5;149:*.pl=0;38;5;48:*.ts=0;38;5;48:*.gz=4;38;5;203:*.so=1;38;5;203:*.cr=0;38;5;48:*.fs=0;38;5;48:*.bz=4;38;5;203:*.ko=1;38;5;203:*.as=0;38;5;48:*.sh=0;38;5;48:*.pp=0;38;5;48:*.el=0;38;5;48:*.py=0;38;5;48:*.lo=0;38;5;243:*.bc=0;38;5;243:*.cc=0;38;5;48:*.pm=0;38;5;48:*.rs=0;38;5;48:*.di=0;38;5;48:*.jl=0;38;5;48:*.rb=0;38;5;48:*.md=0;38;5;185:*.js=0;38;5;48:*.go=0;38;5;48:*.vb=0;38;5;48:*.hi=0;38;5;243:*.kt=0;38;5;48:*.hh=0;38;5;48:*.cs=0;38;5;48:*.mn=0;38;5;48:*.nb=0;38;5;48:*.7z=4;38;5;203:*.ex=0;38;5;48:*.rm=0;38;5;208:*.ps=0;38;5;186:*.td=0;38;5;48:*.la=0;38;5;243:*.aux=0;38;5;243:*.xmp=0;38;5;149:*.mp4=0;38;5;208:*.rpm=4;38;5;203:*.m4a=0;38;5;208:*.zip=4;38;5;203:*.dll=1;38;5;203:*.bcf=0;38;5;243:*.awk=0;38;5;48:*.aif=0;38;5;208:*.zst=4;38;5;203:*.bak=0;38;5;243:*.tgz=4;38;5;203:*.com=1;38;5;203:*.clj=0;38;5;48:*.sxw=0;38;5;186:*.vob=0;38;5;208:*.fsx=0;38;5;48:*.doc=0;38;5;186:*.mkv=0;38;5;208:*.tbz=4;38;5;203:*.ogg=0;38;5;208:*.wma=0;38;5;208:*.mid=0;38;5;208:*.kex=0;38;5;186:*.out=0;38;5;243:*.ltx=0;38;5;48:*.sql=0;38;5;48:*.ppt=0;38;5;186:*.tex=0;38;5;48:*.odp=0;38;5;186:*.log=0;38;5;243:*.arj=4;38;5;203:*.ipp=0;38;5;48:*.sbt=0;38;5;48:*.jpg=0;38;5;208:*.yml=0;38;5;149:*.txt=0;38;5;185:*.csv=0;38;5;185:*.dox=0;38;5;149:*.pro=0;38;5;149:*.bst=0;38;5;149:*TODO=1:*.mir=0;38;5;48:*.bat=1;38;5;203:*.m4v=0;38;5;208:*.pod=0;38;5;48:*.cfg=0;38;5;149:*.pas=0;38;5;48:*.tml=0;38;5;149:*.bib=0;38;5;149:*.ini=0;38;5;149:*.apk=4;38;5;203:*.h++=0;38;5;48:*.pyc=0;38;5;243:*.img=4;38;5;203:*.rst=0;38;5;185:*.swf=0;38;5;208:*.htm=0;38;5;185:*.ttf=0;38;5;208:*.elm=0;38;5;48:*hgrc=0;38;5;149:*.bmp=0;38;5;208:*.fsi=0;38;5;48:*.pgm=0;38;5;208:*.dpr=0;38;5;48:*.xls=0;38;5;186:*.tcl=0;38;5;48:*.mli=0;38;5;48:*.ppm=0;38;5;208:*.bbl=0;38;5;243:*.lua=0;38;5;48:*.asa=0;38;5;48:*.pbm=0;38;5;208:*.avi=0;38;5;208:*.def=0;38;5;48:*.mov=0;38;5;208:*.hxx=0;38;5;48:*.tif=0;38;5;208:*.fon=0;38;5;208:*.zsh=0;38;5;48:*.png=0;38;5;208:*.inc=0;38;5;48:*.jar=4;38;5;203:*.swp=0;38;5;243:*.pid=0;38;5;243:*.gif=0;38;5;208:*.ind=0;38;5;243:*.erl=0;38;5;48:*.ilg=0;38;5;243:*.eps=0;38;5;208:*.tsx=0;38;5;48:*.git=0;38;5;243:*.inl=0;38;5;48:*.rtf=0;38;5;186:*.hpp=0;38;5;48:*.kts=0;38;5;48:*.deb=4;38;5;203:*.svg=0;38;5;208:*.pps=0;38;5;186:*.ps1=0;38;5;48:*.c++=0;38;5;48:*.cpp=0;38;5;48:*.bsh=0;38;5;48:*.php=0;38;5;48:*.exs=0;38;5;48:*.toc=0;38;5;243:*.mp3=0;38;5;208:*.epp=0;38;5;48:*.rar=4;38;5;203:*.wav=0;38;5;208:*.xlr=0;38;5;186:*.tmp=0;38;5;243:*.cxx=0;38;5;48:*.iso=4;38;5;203:*.dmg=4;38;5;203:*.gvy=0;38;5;48:*.bin=4;38;5;203:*.wmv=0;38;5;208:*.blg=0;38;5;243:*.ods=0;38;5;186:*.psd=0;38;5;208:*.mpg=0;38;5;208:*.dot=0;38;5;48:*.cgi=0;38;5;48:*.xml=0;38;5;185:*.htc=0;38;5;48:*.ics=0;38;5;186:*.bz2=4;38;5;203:*.tar=4;38;5;203:*.csx=0;38;5;48:*.ico=0;38;5;208:*.sxi=0;38;5;186:*.nix=0;38;5;149:*.pkg=4;38;5;203:*.bag=4;38;5;203:*.fnt=0;38;5;208:*.idx=0;38;5;243:*.xcf=0;38;5;208:*.exe=1;38;5;203:*.flv=0;38;5;208:*.fls=0;38;5;243:*.otf=0;38;5;208:*.vcd=4;38;5;203:*.vim=0;38;5;48:*.sty=0;38;5;243:*.pdf=0;38;5;186:*.odt=0;38;5;186:*.purs=0;38;5;48:*.h264=0;38;5;208:*.jpeg=0;38;5;208:*.dart=0;38;5;48:*.pptx=0;38;5;186:*.lock=0;38;5;243:*.bash=0;38;5;48:*.rlib=0;38;5;243:*.hgrc=0;38;5;149:*.psm1=0;38;5;48:*.toml=0;38;5;149:*.tbz2=4;38;5;203:*.yaml=0;38;5;149:*.make=0;38;5;149:*.orig=0;38;5;243:*.html=0;38;5;185:*.fish=0;38;5;48:*.diff=0;38;5;48:*.xlsx=0;38;5;186:*.docx=0;38;5;186:*.json=0;38;5;149:*.psd1=0;38;5;48:*.tiff=0;38;5;208:*.flac=0;38;5;208:*.java=0;38;5;48:*.less=0;38;5;48:*.mpeg=0;38;5;208:*.conf=0;38;5;149:*.lisp=0;38;5;48:*.epub=0;38;5;186:*.cabal=0;38;5;48:*.patch=0;38;5;48:*.shtml=0;38;5;185:*.class=0;38;5;243:*.xhtml=0;38;5;185:*.mdown=0;38;5;185:*.dyn_o=0;38;5;243:*.cache=0;38;5;243:*.swift=0;38;5;48:*README=0;38;5;16;48;5;186:*passwd=0;38;5;149:*.ipynb=0;38;5;48:*shadow=0;38;5;149:*.toast=4;38;5;203:*.cmake=0;38;5;149:*.scala=0;38;5;48:*.dyn_hi=0;38;5;243:*.matlab=0;38;5;48:*.config=0;38;5;149:*.gradle=0;38;5;48:*.groovy=0;38;5;48:*.ignore=0;38;5;149:*LICENSE=0;38;5;249:*TODO.md=1:*COPYING=0;38;5;249:*.flake8=0;38;5;149:*INSTALL=0;38;5;16;48;5;186:*setup.py=0;38;5;149:*.gemspec=0;38;5;149:*.desktop=0;38;5;149:*Makefile=0;38;5;149:*Doxyfile=0;38;5;149:*TODO.txt=1:*README.md=0;38;5;16;48;5;186:*.kdevelop=0;38;5;149:*.rgignore=0;38;5;149:*configure=0;38;5;149:*.DS_Store=0;38;5;243:*.fdignore=0;38;5;149:*COPYRIGHT=0;38;5;249:*.markdown=0;38;5;185:*.cmake.in=0;38;5;149:*.gitconfig=0;38;5;149:*INSTALL.md=0;38;5;16;48;5;186:*CODEOWNERS=0;38;5;149:*.gitignore=0;38;5;149:*Dockerfile=0;38;5;149:*SConstruct=0;38;5;149:*.scons_opt=0;38;5;243:*README.txt=0;38;5;16;48;5;186:*SConscript=0;38;5;149:*.localized=0;38;5;243:*.travis.yml=0;38;5;186:*Makefile.in=0;38;5;243:*.gitmodules=0;38;5;149:*LICENSE-MIT=0;38;5;249:*Makefile.am=0;38;5;149:*INSTALL.txt=0;38;5;16;48;5;186:*MANIFEST.in=0;38;5;149:*.synctex.gz=0;38;5;243:*.fdb_latexmk=0;38;5;243:*CONTRIBUTORS=0;38;5;16;48;5;186:*configure.ac=0;38;5;149:*.applescript=0;38;5;48:*appveyor.yml=0;38;5;186:*.clang-format=0;38;5;149:*.gitattributes=0;38;5;149:*LICENSE-APACHE=0;38;5;249:*CMakeCache.txt=0;38;5;243:*CMakeLists.txt=0;38;5;149:*CONTRIBUTORS.md=0;38;5;16;48;5;186:*requirements.txt=0;38;5;149:*CONTRIBUTORS.txt=0;38;5;16;48;5;186:*.sconsign.dblite=0;38;5;243:*package-lock.json=0;38;5;243:*.CFUserTextEncoding=0;38;5;243"),
    };

    let cols = if let Some(col) = width_param {
        col.parse::<u16>().unwrap_or(80)
    } else if let Some((Width(w), Height(_h))) = terminal_size::terminal_size() {
        w
    } else {
        80u16
    };
    let sep = if let Some(separator) = separator_param {
        separator
    } else {
        " │ ".to_string()
    };

    let mut grid = Grid::new(GridOptions {
        direction: Direction::TopToBottom,
        filling: Filling::Text(sep),
    });

    for (_row_index, header, value) in items {
        // only output value if the header name is 'name'
        if header == "name" {
            if color_param {
                if use_grid_icons {
                    let no_ansi = strip_ansi(&value);
                    let path = std::path::Path::new(no_ansi.as_ref());
                    let icon = icon_for_file(path, call.head)?;
                    let ls_colors_style = ls_colors.style_for_path(path);
                    // eprintln!("ls_colors_style: {:?}", &ls_colors_style);

                    let icon_style = match ls_colors_style {
                        Some(c) => c.to_crossterm_style(),
                        None => crossterm::style::ContentStyle::default(),
                    };
                    // eprintln!("icon_style: {:?}", &icon_style);

                    let ansi_style = ls_colors_style
                        .map(Style::to_crossterm_style)
                        .unwrap_or_default();
                    // eprintln!("ansi_style: {:?}", &ansi_style);

                    let item = format!("{} {}", icon_style.apply(icon), ansi_style.apply(value));

                    let mut cell = Cell::from(item);
                    cell.alignment = Alignment::Left;
                    grid.add(cell);
                } else {
                    let style = ls_colors.style_for_path(value.clone());
                    let ansi_style = style.map(Style::to_crossterm_style).unwrap_or_default();
                    let mut cell = Cell::from(ansi_style.apply(value).to_string());
                    cell.alignment = Alignment::Left;
                    grid.add(cell);
                }
            } else {
                let mut cell = Cell::from(value);
                cell.alignment = Alignment::Left;
                grid.add(cell);
            }
        }
    }

    Ok(
        if let Some(grid_display) = grid.fit_into_width(cols as usize) {
            Value::String {
                val: grid_display.to_string(),
                span: call.head,
            }
        } else {
            Value::String {
                val: format!("Couldn't fit grid into {} columns!", cols),
                span: call.head,
            }
        }
        .into_pipeline_data(),
    )
}

fn convert_to_list(
    iter: impl IntoIterator<Item = Value>,
    config: &Config,
    head: Span,
) -> Option<Vec<(usize, String, String)>> {
    let mut iter = iter.into_iter().peekable();

    if let Some(first) = iter.peek() {
        let mut headers = first.columns();

        if !headers.is_empty() {
            headers.insert(0, "#".into());
        }

        let mut data = vec![];

        for (row_num, item) in iter.enumerate() {
            let mut row = vec![row_num.to_string()];

            if headers.is_empty() {
                row.push(item.into_string(", ", config))
            } else {
                for header in headers.iter().skip(1) {
                    let result = match item {
                        Value::Record { .. } => {
                            item.clone().follow_cell_path(&[PathMember::String {
                                val: header.into(),
                                span: head,
                            }])
                        }
                        _ => Ok(item.clone()),
                    };

                    match result {
                        Ok(value) => row.push(value.into_string(", ", config)),
                        Err(_) => row.push(String::new()),
                    }
                }
            }

            data.push(row);
        }

        let mut h: Vec<String> = headers.into_iter().collect();

        // This is just a list
        if h.is_empty() {
            // let's fake the header
            h.push("#".to_string());
            h.push("name".to_string());
        }

        // this tuple is (row_index, header_name, value)
        let mut interleaved = vec![];
        for (i, v) in data.into_iter().enumerate() {
            for (n, s) in v.into_iter().enumerate() {
                if h.len() == 1 {
                    // always get the 1th element since this is a simple list
                    // and we hacked the header above because it was empty
                    // 0th element is an index, 1th element is the value
                    interleaved.push((i, h[1].clone(), s))
                } else {
                    interleaved.push((i, h[n].clone(), s))
                }
            }
        }

        Some(interleaved)
    } else {
        None
    }
}
