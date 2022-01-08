use lscolors::{LsColors, Style};
use nu_color_config::{get_color_config, style_primitive};
use nu_engine::column::get_columns;
use nu_engine::{env_to_string, CallExt};
use nu_protocol::ast::{Call, PathMember};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Config, DataSource, IntoPipelineData, PipelineData, PipelineMetadata, ShellError,
    Signature, Span, StringStream, SyntaxShape, Value, ValueStream,
};
use nu_table::{StyledString, TextStyle, Theme};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use terminal_size::{Height, Width};

const STREAM_PAGE_SIZE: usize = 1000;
const STREAM_TIMEOUT_CHECK_INTERVAL: usize = 100;

#[derive(Clone)]
pub struct Table;

//NOTE: this is not a real implementation :D. It's just a simple one to test with until we port the real one.
impl Command for Table {
    fn name(&self) -> &str {
        "table"
    }

    fn usage(&self) -> &str {
        "Render the table."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("table")
            .named(
                "start_number",
                SyntaxShape::Int,
                "row number to start viewing from",
                Some('n'),
            )
            .category(Category::Viewers)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let head = call.head;
        let ctrlc = engine_state.ctrlc.clone();
        let config = stack.get_config().unwrap_or_default();
        let color_hm = get_color_config(&config);
        let start_num: Option<i64> = call.get_flag(engine_state, stack, "start_number")?;
        let row_offset = start_num.unwrap_or_default() as usize;

        let term_width = if let Some((Width(w), Height(_h))) = terminal_size::terminal_size() {
            (w - 1) as usize
        } else {
            80usize
        };

        match input {
            PipelineData::ByteStream(stream, ..) => Ok(PipelineData::StringStream(
                StringStream::from_stream(
                    stream.map(move |x| {
                        Ok(if x.iter().all(|x| x.is_ascii()) {
                            format!("{}", String::from_utf8_lossy(&x?))
                        } else {
                            format!("{}\n", nu_pretty_hex::pretty_hex(&x?))
                        })
                    }),
                    ctrlc,
                ),
                head,
                None,
            )),
            PipelineData::Value(Value::Binary { val, .. }, ..) => Ok(PipelineData::StringStream(
                StringStream::from_stream(
                    vec![Ok(if val.iter().all(|x| x.is_ascii()) {
                        format!("{}", String::from_utf8_lossy(&val))
                    } else {
                        format!("{}\n", nu_pretty_hex::pretty_hex(&val))
                    })]
                    .into_iter(),
                    ctrlc,
                ),
                head,
                None,
            )),
            PipelineData::Value(Value::List { vals, .. }, ..) => {
                let table = convert_to_table(row_offset, &vals, ctrlc, &config, call.head)?;

                if let Some(table) = table {
                    let result = nu_table::draw_table(&table, term_width, &color_hm, &config);

                    Ok(Value::String {
                        val: result,
                        span: call.head,
                    }
                    .into_pipeline_data())
                } else {
                    Ok(PipelineData::new(call.head))
                }
            }
            PipelineData::ListStream(stream, metadata) => {
                let stream = match metadata {
                    Some(PipelineMetadata {
                        data_source: DataSource::Ls,
                    }) => {
                        let config = config.clone();
                        let ctrlc = ctrlc.clone();

                        let ls_colors = match stack.get_env_var(engine_state, "LS_COLORS") {
                            Some(v) => LsColors::from_string(&env_to_string(
                                "LS_COLORS",
                                v,
                                engine_state,
                                stack,
                                &config,
                            )?),
                            None => LsColors::from_string("pi=0;38;2;0;0;0;48;2;102;217;239:so=0;38;2;0;0;0;48;2;249;38;114:*~=0;38;2;122;112;112:ex=1;38;2;249;38;114:ln=0;38;2;249;38;114:fi=0:or=0;38;2;0;0;0;48;2;255;74;68:di=0;38;2;102;217;239:no=0:mi=0;38;2;0;0;0;48;2;255;74;68:*.r=0;38;2;0;255;135:*.o=0;38;2;122;112;112:*.h=0;38;2;0;255;135:*.p=0;38;2;0;255;135:*.t=0;38;2;0;255;135:*.a=1;38;2;249;38;114:*.z=4;38;2;249;38;114:*.m=0;38;2;0;255;135:*.c=0;38;2;0;255;135:*.d=0;38;2;0;255;135:*.pl=0;38;2;0;255;135:*.pm=0;38;2;0;255;135:*.pp=0;38;2;0;255;135:*.ko=1;38;2;249;38;114:*.ui=0;38;2;166;226;46:*.ps=0;38;2;230;219;116:*.di=0;38;2;0;255;135:*.sh=0;38;2;0;255;135:*.rb=0;38;2;0;255;135:*.cc=0;38;2;0;255;135:*.cr=0;38;2;0;255;135:*.hi=0;38;2;122;112;112:*.xz=4;38;2;249;38;114:*.go=0;38;2;0;255;135:*.bz=4;38;2;249;38;114:*.7z=4;38;2;249;38;114:*.rm=0;38;2;253;151;31:*.cp=0;38;2;0;255;135:*.hh=0;38;2;0;255;135:*.cs=0;38;2;0;255;135:*.el=0;38;2;0;255;135:*.kt=0;38;2;0;255;135:*.py=0;38;2;0;255;135:*.mn=0;38;2;0;255;135:*.hs=0;38;2;0;255;135:*.la=0;38;2;122;112;112:*.vb=0;38;2;0;255;135:*.md=0;38;2;226;209;57:*.rs=0;38;2;0;255;135:*.ml=0;38;2;0;255;135:*.so=1;38;2;249;38;114:*.ts=0;38;2;0;255;135:*.as=0;38;2;0;255;135:*.gz=4;38;2;249;38;114:*.ex=0;38;2;0;255;135:*.jl=0;38;2;0;255;135:*css=0;38;2;0;255;135:*.gv=0;38;2;0;255;135:*.js=0;38;2;0;255;135:*.nb=0;38;2;0;255;135:*.fs=0;38;2;0;255;135:*.lo=0;38;2;122;112;112:*.tml=0;38;2;166;226;46:*.pro=0;38;2;166;226;46:*.pas=0;38;2;0;255;135:*.bin=4;38;2;249;38;114:*.xcf=0;38;2;253;151;31:*.ini=0;38;2;166;226;46:*.fsi=0;38;2;0;255;135:*.ics=0;38;2;230;219;116:*.sbt=0;38;2;0;255;135:*.tar=4;38;2;249;38;114:*.deb=4;38;2;249;38;114:*.cgi=0;38;2;0;255;135:*.xmp=0;38;2;166;226;46:*.hxx=0;38;2;0;255;135:*.cfg=0;38;2;166;226;46:*.bag=4;38;2;249;38;114:*.ppt=0;38;2;230;219;116:*.asa=0;38;2;0;255;135:*.xls=0;38;2;230;219;116:*.htm=0;38;2;226;209;57:*.h++=0;38;2;0;255;135:*.hpp=0;38;2;0;255;135:*.tex=0;38;2;0;255;135:*.tmp=0;38;2;122;112;112:*.erl=0;38;2;0;255;135:*.cxx=0;38;2;0;255;135:*.inl=0;38;2;0;255;135:*.elm=0;38;2;0;255;135:*.kts=0;38;2;0;255;135:*.bz2=4;38;2;249;38;114:*.arj=4;38;2;249;38;114:*.dmg=4;38;2;249;38;114:*.vcd=4;38;2;249;38;114:*.ipp=0;38;2;0;255;135:*TODO=1:*.m4v=0;38;2;253;151;31:*.git=0;38;2;122;112;112:*.pod=0;38;2;0;255;135:*.svg=0;38;2;253;151;31:*.log=0;38;2;122;112;112:*.pgm=0;38;2;253;151;31:*.vim=0;38;2;0;255;135:*.bib=0;38;2;166;226;46:*.rpm=4;38;2;249;38;114:*.mpg=0;38;2;253;151;31:*.dpr=0;38;2;0;255;135:*.aux=0;38;2;122;112;112:*.tsx=0;38;2;0;255;135:*.odt=0;38;2;230;219;116:*.mli=0;38;2;0;255;135:*.ps1=0;38;2;0;255;135:*.cpp=0;38;2;0;255;135:*.flv=0;38;2;253;151;31:*.fsx=0;38;2;0;255;135:*.tif=0;38;2;253;151;31:*.blg=0;38;2;122;112;112:*.sty=0;38;2;122;112;112:*.bak=0;38;2;122;112;112:*.zip=4;38;2;249;38;114:*.sxw=0;38;2;230;219;116:*.clj=0;38;2;0;255;135:*.mkv=0;38;2;253;151;31:*.doc=0;38;2;230;219;116:*.dox=0;38;2;166;226;46:*.swf=0;38;2;253;151;31:*.rst=0;38;2;226;209;57:*.png=0;38;2;253;151;31:*.pid=0;38;2;122;112;112:*.nix=0;38;2;166;226;46:*.aif=0;38;2;253;151;31:*.ogg=0;38;2;253;151;31:*.tgz=4;38;2;249;38;114:*.otf=0;38;2;253;151;31:*.img=4;38;2;249;38;114:*.txt=0;38;2;226;209;57:*.epp=0;38;2;0;255;135:*.jpg=0;38;2;253;151;31:*.c++=0;38;2;0;255;135:*.ppm=0;38;2;253;151;31:*.dll=1;38;2;249;38;114:*.tcl=0;38;2;0;255;135:*.sxi=0;38;2;230;219;116:*.bat=1;38;2;249;38;114:*.mid=0;38;2;253;151;31:*.vob=0;38;2;253;151;31:*.csx=0;38;2;0;255;135:*.idx=0;38;2;122;112;112:*.wma=0;38;2;253;151;31:*hgrc=0;38;2;166;226;46:*.fls=0;38;2;122;112;112:*.lua=0;38;2;0;255;135:*.pkg=4;38;2;249;38;114:*.csv=0;38;2;226;209;57:*.wmv=0;38;2;253;151;31:*.fon=0;38;2;253;151;31:*.avi=0;38;2;253;151;31:*.pps=0;38;2;230;219;116:*.swp=0;38;2;122;112;112:*.iso=4;38;2;249;38;114:*.bcf=0;38;2;122;112;112:*.exe=1;38;2;249;38;114:*.bmp=0;38;2;253;151;31:*.pyc=0;38;2;122;112;112:*.apk=4;38;2;249;38;114:*.ttf=0;38;2;253;151;31:*.yml=0;38;2;166;226;46:*.rar=4;38;2;249;38;114:*.zsh=0;38;2;0;255;135:*.xml=0;38;2;226;209;57:*.htc=0;38;2;0;255;135:*.kex=0;38;2;230;219;116:*.com=1;38;2;249;38;114:*.fnt=0;38;2;253;151;31:*.xlr=0;38;2;230;219;116:*.ods=0;38;2;230;219;116:*.ltx=0;38;2;0;255;135:*.bbl=0;38;2;122;112;112:*.odp=0;38;2;230;219;116:*.ilg=0;38;2;122;112;112:*.exs=0;38;2;0;255;135:*.wav=0;38;2;253;151;31:*.bst=0;38;2;166;226;46:*.pbm=0;38;2;253;151;31:*.sql=0;38;2;0;255;135:*.dot=0;38;2;0;255;135:*.awk=0;38;2;0;255;135:*.tbz=4;38;2;249;38;114:*.toc=0;38;2;122;112;112:*.out=0;38;2;122;112;112:*.mp4=0;38;2;253;151;31:*.ind=0;38;2;122;112;112:*.bsh=0;38;2;0;255;135:*.jar=4;38;2;249;38;114:*.mov=0;38;2;253;151;31:*.ico=0;38;2;253;151;31:*.gvy=0;38;2;0;255;135:*.gif=0;38;2;253;151;31:*.rtf=0;38;2;230;219;116:*.php=0;38;2;0;255;135:*.mp3=0;38;2;253;151;31:*.pdf=0;38;2;230;219;116:*.toml=0;38;2;166;226;46:*.flac=0;38;2;253;151;31:*.conf=0;38;2;166;226;46:*.mpeg=0;38;2;253;151;31:*.hgrc=0;38;2;166;226;46:*.h264=0;38;2;253;151;31:*.yaml=0;38;2;166;226;46:*.json=0;38;2;166;226;46:*.tbz2=4;38;2;249;38;114:*.lock=0;38;2;122;112;112:*.diff=0;38;2;0;255;135:*.xlsx=0;38;2;230;219;116:*.rlib=0;38;2;122;112;112:*.java=0;38;2;0;255;135:*.fish=0;38;2;0;255;135:*.docx=0;38;2;230;219;116:*.html=0;38;2;226;209;57:*.make=0;38;2;166;226;46:*.less=0;38;2;0;255;135:*.pptx=0;38;2;230;219;116:*.epub=0;38;2;230;219;116:*.psm1=0;38;2;0;255;135:*.jpeg=0;38;2;253;151;31:*.lisp=0;38;2;0;255;135:*.orig=0;38;2;122;112;112:*.dart=0;38;2;0;255;135:*.bash=0;38;2;0;255;135:*.purs=0;38;2;0;255;135:*.psd1=0;38;2;0;255;135:*.shtml=0;38;2;226;209;57:*.class=0;38;2;122;112;112:*.cmake=0;38;2;166;226;46:*.cabal=0;38;2;0;255;135:*.scala=0;38;2;0;255;135:*.ipynb=0;38;2;0;255;135:*passwd=0;38;2;166;226;46:*README=0;38;2;0;0;0;48;2;230;219;116:*.swift=0;38;2;0;255;135:*.dyn_o=0;38;2;122;112;112:*shadow=0;38;2;166;226;46:*.patch=0;38;2;0;255;135:*.toast=4;38;2;249;38;114:*.xhtml=0;38;2;226;209;57:*.cache=0;38;2;122;112;112:*.mdown=0;38;2;226;209;57:*COPYING=0;38;2;182;182;182:*TODO.md=1:*.config=0;38;2;166;226;46:*.dyn_hi=0;38;2;122;112;112:*.ignore=0;38;2;166;226;46:*INSTALL=0;38;2;0;0;0;48;2;230;219;116:*LICENSE=0;38;2;182;182;182:*.gradle=0;38;2;0;255;135:*.groovy=0;38;2;0;255;135:*.matlab=0;38;2;0;255;135:*.flake8=0;38;2;166;226;46:*.gemspec=0;38;2;166;226;46:*setup.py=0;38;2;166;226;46:*Makefile=0;38;2;166;226;46:*Doxyfile=0;38;2;166;226;46:*.desktop=0;38;2;166;226;46:*TODO.txt=1:*.kdevelop=0;38;2;166;226;46:*COPYRIGHT=0;38;2;182;182;182:*.cmake.in=0;38;2;166;226;46:*.rgignore=0;38;2;166;226;46:*README.md=0;38;2;0;0;0;48;2;230;219;116:*.markdown=0;38;2;226;209;57:*configure=0;38;2;166;226;46:*.fdignore=0;38;2;166;226;46:*Dockerfile=0;38;2;166;226;46:*README.txt=0;38;2;0;0;0;48;2;230;219;116:*INSTALL.md=0;38;2;0;0;0;48;2;230;219;116:*.gitignore=0;38;2;166;226;46:*SConscript=0;38;2;166;226;46:*.scons_opt=0;38;2;122;112;112:*SConstruct=0;38;2;166;226;46:*CODEOWNERS=0;38;2;166;226;46:*.gitconfig=0;38;2;166;226;46:*.synctex.gz=0;38;2;122;112;112:*.gitmodules=0;38;2;166;226;46:*Makefile.am=0;38;2;166;226;46:*LICENSE-MIT=0;38;2;182;182;182:*Makefile.in=0;38;2;122;112;112:*MANIFEST.in=0;38;2;166;226;46:*.travis.yml=0;38;2;230;219;116:*CONTRIBUTORS=0;38;2;0;0;0;48;2;230;219;116:*configure.ac=0;38;2;166;226;46:*.applescript=0;38;2;0;255;135:*appveyor.yml=0;38;2;230;219;116:*.fdb_latexmk=0;38;2;122;112;112:*.clang-format=0;38;2;166;226;46:*LICENSE-APACHE=0;38;2;182;182;182:*INSTALL.md.txt=0;38;2;0;0;0;48;2;230;219;116:*CMakeLists.txt=0;38;2;166;226;46:*.gitattributes=0;38;2;166;226;46:*CMakeCache.txt=0;38;2;122;112;112:*CONTRIBUTORS.md=0;38;2;0;0;0;48;2;230;219;116:*CONTRIBUTORS.txt=0;38;2;0;0;0;48;2;230;219;116:*.sconsign.dblite=0;38;2;122;112;112:*requirements.txt=0;38;2;166;226;46:*package-lock.json=0;38;2;122;112;112"),
                        };

                        ValueStream::from_stream(
                            stream.map(move |mut x| match &mut x {
                                Value::Record { cols, vals, .. } => {
                                    let mut idx = 0;

                                    while idx < cols.len() {
                                        if cols[idx] == "name" {
                                            if let Some(Value::String { val: path, span }) =
                                                vals.get(idx)
                                            {
                                                match std::fs::symlink_metadata(&path) {
                                                    Ok(metadata) => {
                                                        let style = ls_colors
                                                            .style_for_path_with_metadata(
                                                                path.clone(),
                                                                Some(&metadata),
                                                            );
                                                        let ansi_style = style
                                                            .map(Style::to_crossterm_style)
                                                            .unwrap_or_default();
                                                        let use_ls_colors = config.use_ls_colors;

                                                        if use_ls_colors {
                                                            vals[idx] = Value::String {
                                                                val: ansi_style
                                                                    .apply(path)
                                                                    .to_string(),
                                                                span: *span,
                                                            };
                                                        }
                                                    }
                                                    Err(_) => {
                                                        let style =
                                                            ls_colors.style_for_path(path.clone());
                                                        let ansi_style = style
                                                            .map(Style::to_crossterm_style)
                                                            .unwrap_or_default();
                                                        let use_ls_colors = config.use_ls_colors;

                                                        if use_ls_colors {
                                                            vals[idx] = Value::String {
                                                                val: ansi_style
                                                                    .apply(path)
                                                                    .to_string(),
                                                                span: *span,
                                                            };
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        idx += 1;
                                    }

                                    x
                                }
                                _ => x,
                            }),
                            ctrlc,
                        )
                    }
                    _ => stream,
                };

                let head = call.head;

                Ok(PipelineData::StringStream(
                    StringStream::from_stream(
                        PagingTableCreator {
                            row_offset,
                            config,
                            ctrlc: ctrlc.clone(),
                            head,
                            stream,
                        },
                        ctrlc,
                    ),
                    head,
                    None,
                ))
            }
            PipelineData::Value(Value::Record { cols, vals, .. }, ..) => {
                let mut output = vec![];

                for (c, v) in cols.into_iter().zip(vals.into_iter()) {
                    output.push(vec![
                        StyledString {
                            contents: c,
                            style: TextStyle::default_field(),
                        },
                        StyledString {
                            contents: v.into_abbreviated_string(&config),
                            style: TextStyle::default(),
                        },
                    ])
                }

                let table = nu_table::Table {
                    headers: vec![],
                    data: output,
                    theme: load_theme_from_config(&config),
                };

                let result = nu_table::draw_table(&table, term_width, &color_hm, &config);

                Ok(Value::String {
                    val: result,
                    span: call.head,
                }
                .into_pipeline_data())
            }
            PipelineData::Value(Value::Error { error }, ..) => Err(error),
            PipelineData::Value(Value::CustomValue { val, span }, ..) => {
                let base_pipeline = val.to_base_value(span)?.into_pipeline_data();
                self.run(engine_state, stack, call, base_pipeline)
            }
            x => Ok(x),
        }
    }
}

fn convert_to_table(
    row_offset: usize,
    input: &[Value],
    ctrlc: Option<Arc<AtomicBool>>,
    config: &Config,
    head: Span,
) -> Result<Option<nu_table::Table>, ShellError> {
    let mut headers = get_columns(input);
    let mut input = input.iter().peekable();
    let color_hm = get_color_config(config);
    let float_precision = config.float_precision as usize;

    if input.peek().is_some() {
        if !headers.is_empty() {
            headers.insert(0, "#".into());
        }

        // Vec of Vec of String1, String2 where String1 is datatype and String2 is value
        let mut data: Vec<Vec<(String, String)>> = Vec::new();

        for (row_num, item) in input.enumerate() {
            if let Some(ctrlc) = &ctrlc {
                if ctrlc.load(Ordering::SeqCst) {
                    return Ok(None);
                }
            }
            if let Value::Error { error } = item {
                return Err(error.clone());
            }
            // String1 = datatype, String2 = value as string
            let mut row: Vec<(String, String)> =
                vec![("string".to_string(), (row_num + row_offset).to_string())];

            if headers.is_empty() {
                // if header row is empty, this is probably a list so format it that way
                row.push(("list".to_string(), item.into_abbreviated_string(config)))
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
                        Ok(value) => row.push((
                            (&value.get_type()).to_string(),
                            value.into_abbreviated_string(config),
                        )),
                        Err(_) => row.push(("empty".to_string(), "❎".into())),
                    }
                }
            }

            data.push(row);
        }

        Ok(Some(nu_table::Table {
            headers: headers
                .into_iter()
                .map(|x| StyledString {
                    contents: x,
                    style: TextStyle {
                        alignment: nu_table::Alignment::Center,
                        color_style: Some(color_hm["header"]),
                    },
                })
                .collect(),
            data: data
                .into_iter()
                .map(|x| {
                    x.into_iter()
                        .enumerate()
                        .map(|(col, y)| {
                            if col == 0 {
                                StyledString {
                                    contents: y.1,
                                    style: TextStyle {
                                        alignment: nu_table::Alignment::Right,
                                        color_style: Some(color_hm["row_index"]),
                                    },
                                }
                            } else if &y.0 == "float" {
                                // set dynamic precision from config
                                let precise_number =
                                    match convert_with_precision(&y.1, float_precision) {
                                        Ok(num) => num,
                                        Err(e) => e.to_string(),
                                    };
                                StyledString {
                                    contents: precise_number,
                                    style: style_primitive(&y.0, &color_hm),
                                }
                            } else {
                                StyledString {
                                    contents: y.1,
                                    style: style_primitive(&y.0, &color_hm),
                                }
                            }
                        })
                        .collect::<Vec<StyledString>>()
                })
                .collect(),
            theme: load_theme_from_config(config),
        }))
    } else {
        Ok(None)
    }
}

fn convert_with_precision(val: &str, precision: usize) -> Result<String, ShellError> {
    // vall will always be a f64 so convert it with precision formatting
    let val_float = match val.trim().parse::<f64>() {
        Ok(f) => f,
        Err(e) => {
            return Err(ShellError::LabeledError(
                format!("error converting string [{}] to f64", &val),
                e.to_string(),
            ));
        }
    };
    Ok(format!("{:.prec$}", val_float, prec = precision))
}

struct PagingTableCreator {
    head: Span,
    stream: ValueStream,
    ctrlc: Option<Arc<AtomicBool>>,
    config: Config,
    row_offset: usize,
}

impl Iterator for PagingTableCreator {
    type Item = Result<String, ShellError>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut batch = vec![];

        let start_time = Instant::now();

        let mut idx = 0;

        // Pull from stream until time runs out or we have enough items
        for item in self.stream.by_ref() {
            batch.push(item);
            idx += 1;

            if idx % STREAM_TIMEOUT_CHECK_INTERVAL == 0 {
                let end_time = Instant::now();

                // If we've been buffering over a second, go ahead and send out what we have so far
                if (end_time - start_time).as_secs() >= 1 {
                    break;
                }
            }

            if idx == STREAM_PAGE_SIZE {
                break;
            }

            if let Some(ctrlc) = &self.ctrlc {
                if ctrlc.load(Ordering::SeqCst) {
                    break;
                }
            }
        }

        let color_hm = get_color_config(&self.config);

        let term_width = if let Some((Width(w), Height(_h))) = terminal_size::terminal_size() {
            (w - 1) as usize
        } else {
            80usize
        };

        let table = convert_to_table(
            self.row_offset,
            &batch,
            self.ctrlc.clone(),
            &self.config,
            self.head,
        );
        self.row_offset += idx;

        match table {
            Ok(Some(table)) => {
                let result = nu_table::draw_table(&table, term_width, &color_hm, &self.config);

                Some(Ok(result))
            }
            Err(err) => Some(Err(err)),
            _ => None,
        }
    }
}

fn load_theme_from_config(config: &Config) -> Theme {
    match config.table_mode.as_str() {
        "basic" => nu_table::Theme::basic(),
        "compact" => nu_table::Theme::compact(),
        "compact_double" => nu_table::Theme::compact_double(),
        "light" => nu_table::Theme::light(),
        "with_love" => nu_table::Theme::with_love(),
        "rounded" => nu_table::Theme::rounded(),
        "reinforced" => nu_table::Theme::reinforced(),
        "heavy" => nu_table::Theme::heavy(),
        "none" => nu_table::Theme::none(),
        _ => nu_table::Theme::rounded(),
    }
}
