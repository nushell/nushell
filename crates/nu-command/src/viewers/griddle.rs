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

fn strip_ansi(astring: &str) -> String {
    if let Ok(bytes) = strip_ansi_escapes::strip(astring) {
        String::from_utf8_lossy(&bytes).to_string()
    } else {
        astring.to_string()
    }
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
        None => LsColors::from_string("pi=0;38;2;0;0;0;48;2;102;217;239:so=0;38;2;0;0;0;48;2;249;38;114:*~=0;38;2;122;112;112:ex=1;38;2;249;38;114:ln=0;38;2;249;38;114:fi=0:or=0;38;2;0;0;0;48;2;255;74;68:di=0;38;2;102;217;239:no=0:mi=0;38;2;0;0;0;48;2;255;74;68:*.r=0;38;2;0;255;135:*.o=0;38;2;122;112;112:*.h=0;38;2;0;255;135:*.p=0;38;2;0;255;135:*.t=0;38;2;0;255;135:*.a=1;38;2;249;38;114:*.z=4;38;2;249;38;114:*.m=0;38;2;0;255;135:*.c=0;38;2;0;255;135:*.d=0;38;2;0;255;135:*.pl=0;38;2;0;255;135:*.pm=0;38;2;0;255;135:*.pp=0;38;2;0;255;135:*.ko=1;38;2;249;38;114:*.ui=0;38;2;166;226;46:*.ps=0;38;2;230;219;116:*.di=0;38;2;0;255;135:*.sh=0;38;2;0;255;135:*.rb=0;38;2;0;255;135:*.cc=0;38;2;0;255;135:*.cr=0;38;2;0;255;135:*.hi=0;38;2;122;112;112:*.xz=4;38;2;249;38;114:*.go=0;38;2;0;255;135:*.bz=4;38;2;249;38;114:*.7z=4;38;2;249;38;114:*.rm=0;38;2;253;151;31:*.cp=0;38;2;0;255;135:*.hh=0;38;2;0;255;135:*.cs=0;38;2;0;255;135:*.el=0;38;2;0;255;135:*.kt=0;38;2;0;255;135:*.py=0;38;2;0;255;135:*.mn=0;38;2;0;255;135:*.hs=0;38;2;0;255;135:*.la=0;38;2;122;112;112:*.vb=0;38;2;0;255;135:*.md=0;38;2;226;209;57:*.rs=0;38;2;0;255;135:*.ml=0;38;2;0;255;135:*.so=1;38;2;249;38;114:*.ts=0;38;2;0;255;135:*.as=0;38;2;0;255;135:*.gz=4;38;2;249;38;114:*.ex=0;38;2;0;255;135:*.jl=0;38;2;0;255;135:*css=0;38;2;0;255;135:*.gv=0;38;2;0;255;135:*.js=0;38;2;0;255;135:*.nb=0;38;2;0;255;135:*.fs=0;38;2;0;255;135:*.lo=0;38;2;122;112;112:*.tml=0;38;2;166;226;46:*.pro=0;38;2;166;226;46:*.pas=0;38;2;0;255;135:*.bin=4;38;2;249;38;114:*.xcf=0;38;2;253;151;31:*.ini=0;38;2;166;226;46:*.fsi=0;38;2;0;255;135:*.ics=0;38;2;230;219;116:*.sbt=0;38;2;0;255;135:*.tar=4;38;2;249;38;114:*.deb=4;38;2;249;38;114:*.cgi=0;38;2;0;255;135:*.xmp=0;38;2;166;226;46:*.hxx=0;38;2;0;255;135:*.cfg=0;38;2;166;226;46:*.bag=4;38;2;249;38;114:*.ppt=0;38;2;230;219;116:*.asa=0;38;2;0;255;135:*.xls=0;38;2;230;219;116:*.htm=0;38;2;226;209;57:*.h++=0;38;2;0;255;135:*.hpp=0;38;2;0;255;135:*.tex=0;38;2;0;255;135:*.tmp=0;38;2;122;112;112:*.erl=0;38;2;0;255;135:*.cxx=0;38;2;0;255;135:*.inl=0;38;2;0;255;135:*.elm=0;38;2;0;255;135:*.kts=0;38;2;0;255;135:*.bz2=4;38;2;249;38;114:*.arj=4;38;2;249;38;114:*.dmg=4;38;2;249;38;114:*.vcd=4;38;2;249;38;114:*.ipp=0;38;2;0;255;135:*TODO=1:*.m4v=0;38;2;253;151;31:*.git=0;38;2;122;112;112:*.pod=0;38;2;0;255;135:*.svg=0;38;2;253;151;31:*.log=0;38;2;122;112;112:*.pgm=0;38;2;253;151;31:*.vim=0;38;2;0;255;135:*.bib=0;38;2;166;226;46:*.rpm=4;38;2;249;38;114:*.mpg=0;38;2;253;151;31:*.dpr=0;38;2;0;255;135:*.aux=0;38;2;122;112;112:*.tsx=0;38;2;0;255;135:*.odt=0;38;2;230;219;116:*.mli=0;38;2;0;255;135:*.ps1=0;38;2;0;255;135:*.cpp=0;38;2;0;255;135:*.flv=0;38;2;253;151;31:*.fsx=0;38;2;0;255;135:*.tif=0;38;2;253;151;31:*.blg=0;38;2;122;112;112:*.sty=0;38;2;122;112;112:*.bak=0;38;2;122;112;112:*.zip=4;38;2;249;38;114:*.sxw=0;38;2;230;219;116:*.clj=0;38;2;0;255;135:*.mkv=0;38;2;253;151;31:*.doc=0;38;2;230;219;116:*.dox=0;38;2;166;226;46:*.swf=0;38;2;253;151;31:*.rst=0;38;2;226;209;57:*.png=0;38;2;253;151;31:*.pid=0;38;2;122;112;112:*.nix=0;38;2;166;226;46:*.aif=0;38;2;253;151;31:*.ogg=0;38;2;253;151;31:*.tgz=4;38;2;249;38;114:*.otf=0;38;2;253;151;31:*.img=4;38;2;249;38;114:*.txt=0;38;2;226;209;57:*.epp=0;38;2;0;255;135:*.jpg=0;38;2;253;151;31:*.c++=0;38;2;0;255;135:*.ppm=0;38;2;253;151;31:*.dll=1;38;2;249;38;114:*.tcl=0;38;2;0;255;135:*.sxi=0;38;2;230;219;116:*.bat=1;38;2;249;38;114:*.mid=0;38;2;253;151;31:*.vob=0;38;2;253;151;31:*.csx=0;38;2;0;255;135:*.idx=0;38;2;122;112;112:*.wma=0;38;2;253;151;31:*hgrc=0;38;2;166;226;46:*.fls=0;38;2;122;112;112:*.lua=0;38;2;0;255;135:*.pkg=4;38;2;249;38;114:*.csv=0;38;2;226;209;57:*.wmv=0;38;2;253;151;31:*.fon=0;38;2;253;151;31:*.avi=0;38;2;253;151;31:*.pps=0;38;2;230;219;116:*.swp=0;38;2;122;112;112:*.iso=4;38;2;249;38;114:*.bcf=0;38;2;122;112;112:*.exe=1;38;2;249;38;114:*.bmp=0;38;2;253;151;31:*.pyc=0;38;2;122;112;112:*.apk=4;38;2;249;38;114:*.ttf=0;38;2;253;151;31:*.yml=0;38;2;166;226;46:*.rar=4;38;2;249;38;114:*.zsh=0;38;2;0;255;135:*.xml=0;38;2;226;209;57:*.htc=0;38;2;0;255;135:*.kex=0;38;2;230;219;116:*.com=1;38;2;249;38;114:*.fnt=0;38;2;253;151;31:*.xlr=0;38;2;230;219;116:*.ods=0;38;2;230;219;116:*.ltx=0;38;2;0;255;135:*.bbl=0;38;2;122;112;112:*.odp=0;38;2;230;219;116:*.ilg=0;38;2;122;112;112:*.exs=0;38;2;0;255;135:*.wav=0;38;2;253;151;31:*.bst=0;38;2;166;226;46:*.pbm=0;38;2;253;151;31:*.sql=0;38;2;0;255;135:*.dot=0;38;2;0;255;135:*.awk=0;38;2;0;255;135:*.tbz=4;38;2;249;38;114:*.toc=0;38;2;122;112;112:*.out=0;38;2;122;112;112:*.mp4=0;38;2;253;151;31:*.ind=0;38;2;122;112;112:*.bsh=0;38;2;0;255;135:*.jar=4;38;2;249;38;114:*.mov=0;38;2;253;151;31:*.ico=0;38;2;253;151;31:*.gvy=0;38;2;0;255;135:*.gif=0;38;2;253;151;31:*.rtf=0;38;2;230;219;116:*.php=0;38;2;0;255;135:*.mp3=0;38;2;253;151;31:*.pdf=0;38;2;230;219;116:*.toml=0;38;2;166;226;46:*.flac=0;38;2;253;151;31:*.conf=0;38;2;166;226;46:*.mpeg=0;38;2;253;151;31:*.hgrc=0;38;2;166;226;46:*.h264=0;38;2;253;151;31:*.yaml=0;38;2;166;226;46:*.json=0;38;2;166;226;46:*.tbz2=4;38;2;249;38;114:*.lock=0;38;2;122;112;112:*.diff=0;38;2;0;255;135:*.xlsx=0;38;2;230;219;116:*.rlib=0;38;2;122;112;112:*.java=0;38;2;0;255;135:*.fish=0;38;2;0;255;135:*.docx=0;38;2;230;219;116:*.html=0;38;2;226;209;57:*.make=0;38;2;166;226;46:*.less=0;38;2;0;255;135:*.pptx=0;38;2;230;219;116:*.epub=0;38;2;230;219;116:*.psm1=0;38;2;0;255;135:*.jpeg=0;38;2;253;151;31:*.lisp=0;38;2;0;255;135:*.orig=0;38;2;122;112;112:*.dart=0;38;2;0;255;135:*.bash=0;38;2;0;255;135:*.purs=0;38;2;0;255;135:*.psd1=0;38;2;0;255;135:*.shtml=0;38;2;226;209;57:*.class=0;38;2;122;112;112:*.cmake=0;38;2;166;226;46:*.cabal=0;38;2;0;255;135:*.scala=0;38;2;0;255;135:*.ipynb=0;38;2;0;255;135:*passwd=0;38;2;166;226;46:*README=0;38;2;0;0;0;48;2;230;219;116:*.swift=0;38;2;0;255;135:*.dyn_o=0;38;2;122;112;112:*shadow=0;38;2;166;226;46:*.patch=0;38;2;0;255;135:*.toast=4;38;2;249;38;114:*.xhtml=0;38;2;226;209;57:*.cache=0;38;2;122;112;112:*.mdown=0;38;2;226;209;57:*COPYING=0;38;2;182;182;182:*TODO.md=1:*.config=0;38;2;166;226;46:*.dyn_hi=0;38;2;122;112;112:*.ignore=0;38;2;166;226;46:*INSTALL=0;38;2;0;0;0;48;2;230;219;116:*LICENSE=0;38;2;182;182;182:*.gradle=0;38;2;0;255;135:*.groovy=0;38;2;0;255;135:*.matlab=0;38;2;0;255;135:*.flake8=0;38;2;166;226;46:*.gemspec=0;38;2;166;226;46:*setup.py=0;38;2;166;226;46:*Makefile=0;38;2;166;226;46:*Doxyfile=0;38;2;166;226;46:*.desktop=0;38;2;166;226;46:*TODO.txt=1:*.kdevelop=0;38;2;166;226;46:*COPYRIGHT=0;38;2;182;182;182:*.cmake.in=0;38;2;166;226;46:*.rgignore=0;38;2;166;226;46:*README.md=0;38;2;0;0;0;48;2;230;219;116:*.markdown=0;38;2;226;209;57:*configure=0;38;2;166;226;46:*.fdignore=0;38;2;166;226;46:*Dockerfile=0;38;2;166;226;46:*README.txt=0;38;2;0;0;0;48;2;230;219;116:*INSTALL.md=0;38;2;0;0;0;48;2;230;219;116:*.gitignore=0;38;2;166;226;46:*SConscript=0;38;2;166;226;46:*.scons_opt=0;38;2;122;112;112:*SConstruct=0;38;2;166;226;46:*CODEOWNERS=0;38;2;166;226;46:*.gitconfig=0;38;2;166;226;46:*.synctex.gz=0;38;2;122;112;112:*.gitmodules=0;38;2;166;226;46:*Makefile.am=0;38;2;166;226;46:*LICENSE-MIT=0;38;2;182;182;182:*Makefile.in=0;38;2;122;112;112:*MANIFEST.in=0;38;2;166;226;46:*.travis.yml=0;38;2;230;219;116:*CONTRIBUTORS=0;38;2;0;0;0;48;2;230;219;116:*configure.ac=0;38;2;166;226;46:*.applescript=0;38;2;0;255;135:*appveyor.yml=0;38;2;230;219;116:*.fdb_latexmk=0;38;2;122;112;112:*.clang-format=0;38;2;166;226;46:*LICENSE-APACHE=0;38;2;182;182;182:*INSTALL.md.txt=0;38;2;0;0;0;48;2;230;219;116:*CMakeLists.txt=0;38;2;166;226;46:*.gitattributes=0;38;2;166;226;46:*CMakeCache.txt=0;38;2;122;112;112:*CONTRIBUTORS.md=0;38;2;0;0;0;48;2;230;219;116:*CONTRIBUTORS.txt=0;38;2;0;0;0;48;2;230;219;116:*.sconsign.dblite=0;38;2;122;112;112:*requirements.txt=0;38;2;166;226;46:*package-lock.json=0;38;2;122;112;112"),
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
                    let path = std::path::Path::new(&no_ansi);
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

                    let item = format!(
                        "{} {}",
                        icon_style.apply(icon).to_string(),
                        ansi_style.apply(value).to_string()
                    );

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
