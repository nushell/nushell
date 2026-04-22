use nu_ansi_term::{Color, Rgb, Style};
use nu_engine::command_prelude::*;
use std::fmt::Write;

const GRADIENT_ATLAST: [Rgb; 3] = [
    Rgb::new(0xfe, 0xac, 0x5e),
    Rgb::new(0xc7, 0x9d, 0xd0),
    Rgb::new(0x4b, 0xc0, 0xc8),
];

const GRADIENT_CRYSTAL: [Rgb; 2] = [Rgb::new(0xbd, 0xff, 0xf3), Rgb::new(0x5a, 0xc2, 0x9a)];

const GRADIENT_TEEN: [Rgb; 3] = [
    Rgb::new(0x77, 0xa1, 0xd3),
    Rgb::new(0x79, 0xcb, 0xca),
    Rgb::new(0xe6, 0x84, 0xae),
];

const GRADIENT_MIND: [Rgb; 3] = [
    Rgb::new(0x47, 0x3b, 0x7b),
    Rgb::new(0x35, 0x84, 0xa7),
    Rgb::new(0x30, 0xd2, 0xbe),
];

const GRADIENT_MORNING: [Rgb; 2] = [Rgb::new(0xff, 0x5f, 0x6d), Rgb::new(0xff, 0xc3, 0x71)];

const GRADIENT_VICE: [Rgb; 2] = [Rgb::new(0x5e, 0xe7, 0xdf), Rgb::new(0xb4, 0x90, 0xca)];

const GRADIENT_PASSION: [Rgb; 2] = [Rgb::new(0xf4, 0x3b, 0x47), Rgb::new(0x45, 0x3a, 0x94)];

const GRADIENT_FRUIT: [Rgb; 2] = [Rgb::new(0xff, 0x4e, 0x50), Rgb::new(0xf9, 0xd4, 0x23)];

const GRADIENT_RETRO: [Rgb; 9] = [
    Rgb::new(0x3f, 0x51, 0xb1),
    Rgb::new(0x5a, 0x55, 0xae),
    Rgb::new(0x7b, 0x5f, 0xac),
    Rgb::new(0x8f, 0x6a, 0xae),
    Rgb::new(0xa8, 0x6a, 0xa4),
    Rgb::new(0xc5, 0x6a, 0x9b),
    Rgb::new(0xcc, 0x6b, 0x8e),
    Rgb::new(0xf1, 0x82, 0x71),
    Rgb::new(0xf3, 0xa4, 0x69),
];

const GRADIENT_SUMMER: [Rgb; 2] = [Rgb::new(0xfd, 0xbb, 0x2d), Rgb::new(0x22, 0xc1, 0xc3)];

const GRADIENT_RAINBOW: [Rgb; 6] = [
    Rgb::new(189, 19, 84),
    Rgb::new(228, 108, 33),
    Rgb::new(226, 166, 29),
    Rgb::new(46, 163, 44),
    Rgb::new(54, 83, 238),
    Rgb::new(87, 32, 131),
];

const GRADIENT_PASTEL: [Rgb; 5] = [
    Rgb::new(255, 223, 204),
    Rgb::new(255, 243, 219),
    Rgb::new(203, 235, 195),
    Rgb::new(173, 215, 219),
    Rgb::new(137, 143, 173),
];

const GRADIENT_MONSOON: [Rgb; 6] = [
    Rgb::new(181, 199, 204),
    Rgb::new(139, 161, 173),
    Rgb::new(88, 123, 137),
    Rgb::new(36, 76, 102),
    Rgb::new(64, 125, 108),
    Rgb::new(125, 178, 144),
];

const GRADIENT_FOREST: [Rgb; 5] = [
    Rgb::new(69, 55, 48),
    Rgb::new(130, 94, 65),
    Rgb::new(44, 62, 57),
    Rgb::new(65, 90, 69),
    Rgb::new(108, 112, 88),
];

const GRADIENT_INSTAGRAM: [Rgb; 3] = [
    Rgb::new(0x83, 0x3a, 0xb4),
    Rgb::new(0xfd, 0x1d, 0x1d),
    Rgb::new(0xfc, 0xb0, 0x45),
];

static NAMED_GRADIENTS: [(&str, &[Rgb]); 15] = [
    ("atlast", &GRADIENT_ATLAST),
    ("crystal", &GRADIENT_CRYSTAL),
    ("teen", &GRADIENT_TEEN),
    ("mind", &GRADIENT_MIND),
    ("morning", &GRADIENT_MORNING),
    ("vice", &GRADIENT_VICE),
    ("passion", &GRADIENT_PASSION),
    ("fruit", &GRADIENT_FRUIT),
    ("retro", &GRADIENT_RETRO),
    ("summer", &GRADIENT_SUMMER),
    ("rainbow", &GRADIENT_RAINBOW),
    ("pastel", &GRADIENT_PASTEL),
    ("monsoon", &GRADIENT_MONSOON),
    ("forest", &GRADIENT_FOREST),
    ("instagram", &GRADIENT_INSTAGRAM),
];

/// `ansi gradient` command implementation.
///
/// Supports raw foreground/background color gradients and named palettes.
#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "ansi gradient"
    }

    fn signature(&self) -> Signature {
        Signature::build("ansi gradient")
            .named(
                "fgstart",
                SyntaxShape::String,
                "Foreground gradient start color in hex (0x123456).",
                Some('a'),
            )
            .named(
                "fgend",
                SyntaxShape::String,
                "Foreground gradient end color in hex.",
                Some('b'),
            )
            .named(
                "fgnamed",
                SyntaxShape::String,
                "Named foreground gradient.",
                Some('F'),
            )
            .named(
                "bgstart",
                SyntaxShape::String,
                "Background gradient start color in hex.",
                Some('c'),
            )
            .named(
                "bgend",
                SyntaxShape::String,
                "Background gradient end color in hex.",
                Some('d'),
            )
            .named(
                "bgnamed",
                SyntaxShape::String,
                "Named background gradient.",
                Some('B'),
            )
            .switch(
                "list",
                "List available named gradients and show an example.",
                Some('l'),
            )
            .rest(
                "cell path",
                SyntaxShape::CellPath,
                "For a data structure input, add a gradient to strings at the given cell paths.",
            )
            .input_output_types(vec![
                (Type::String, Type::String),
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::String)),
                ),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
                (Type::Nothing, Type::String),
            ])
            .allow_variants_without_examples(true)
            .category(Category::Platform)
    }

    fn description(&self) -> &str {
        "Add a color gradient (using ANSI color codes) to the given string."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        operate(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "draw text in a gradient with foreground start and end colors",
                example: "'Hello, Nushell! This is a gradient.' | ansi gradient --fgstart '0x40c9ff' --fgend '0xe81cff'",
                result: Some(Value::test_string(
                    "\u{1b}[38;2;64;201;255mH\u{1b}[38;2;68;195;254me\u{1b}[38;2;73;190;254ml\u{1b}[38;2;77;185;254ml\u{1b}[38;2;82;181;254mo\u{1b}[38;2;87;176;254m,\u{1b}[38;2;92;170;254m \u{1b}[38;2;97;165;255mN\u{1b}[38;2;102;161;254mu\u{1b}[38;2;106;156;254ms\u{1b}[38;2;111;151;254mh\u{1b}[38;2;115;145;254me\u{1b}[38;2;121;141;254ml\u{1b}[38;2;126;136;254ml\u{1b}[38;2;130;131;255m!\u{1b}[38;2;135;126;254m \u{1b}[38;2;140;121;254mT\u{1b}[38;2;144;116;254mh\u{1b}[38;2;150;111;254mi\u{1b}[38;2;154;106;254ms\u{1b}[38;2;159;102;254m \u{1b}[38;2;164;96;254mi\u{1b}[38;2;168;91;254ms\u{1b}[38;2;173;86;254m \u{1b}[38;2;179;82;254ma\u{1b}[38;2;183;77;254m \u{1b}[38;2;188;71;254mg\u{1b}[38;2;192;66;254mr\u{1b}[38;2;197;62;254ma\u{1b}[38;2;202;57;254md\u{1b}[38;2;207;52;254mi\u{1b}[38;2;212;46;254me\u{1b}[38;2;217;42;254mn\u{1b}[38;2;221;37;254mt\u{1b}[38;2;226;32;254m.\u{1b}[0m",
                )),
            },
            Example {
                description: "draw text in a gradient with foreground start and end colors and background start and end colors",
                example: "'Hello, Nushell! This is a gradient.' | ansi gradient --fgstart '0x40c9ff' --fgend '0xe81cff' --bgstart '0xe81cff' --bgend '0x40c9ff'",
                result: Some(Value::test_string(
                    "\u{1b}[48;2;232;28;255;38;2;64;201;255mH\u{1b}[48;2;226;32;254;38;2;68;195;254me\u{1b}[48;2;221;37;254;38;2;73;190;254ml\u{1b}[48;2;217;42;254;38;2;77;185;254ml\u{1b}[48;2;212;46;254;38;2;82;181;254mo\u{1b}[48;2;207;52;254;38;2;87;176;254m,\u{1b}[48;2;202;57;254;38;2;92;170;254m \u{1b}[48;2;197;62;255;38;2;97;165;255mN\u{1b}[48;2;192;66;254;38;2;102;161;254mu\u{1b}[48;2;188;71;254;38;2;106;156;254ms\u{1b}[48;2;183;77;254;38;2;111;151;254mh\u{1b}[48;2;179;82;254;38;2;115;145;254me\u{1b}[48;2;173;86;254;38;2;121;141;254ml\u{1b}[48;2;168;91;254;38;2;126;136;254ml\u{1b}[48;2;164;96;255;38;2;130;131;255m!\u{1b}[48;2;159;101;254;38;2;135;126;254m \u{1b}[48;2;154;106;254;38;2;140;121;254mT\u{1b}[48;2;150;111;254;38;2;144;116;254mh\u{1b}[48;2;144;116;254;38;2;150;111;254mi\u{1b}[48;2;140;121;254;38;2;154;106;254ms\u{1b}[48;2;135;125;254;38;2;159;102;254m \u{1b}[48;2;130;131;254;38;2;164;96;254mi\u{1b}[48;2;126;136;254;38;2;168;91;254ms\u{1b}[48;2;121;141;254;38;2;173;86;254m \u{1b}[48;2;115;145;254;38;2;179;82;254ma\u{1b}[48;2;111;150;254;38;2;183;77;254m \u{1b}[48;2;106;156;254;38;2;188;71;254mg\u{1b}[48;2;102;161;254;38;2;192;66;254mr\u{1b}[48;2;97;165;254;38;2;197;62;254ma\u{1b}[48;2;92;170;254;38;2;202;57;254md\u{1b}[48;2;87;175;254;38;2;207;52;254mi\u{1b}[48;2;82;181;254;38;2;212;46;254me\u{1b}[48;2;77;185;254;38;2;217;42;254mn\u{1b}[48;2;73;190;254;38;2;221;37;254mt\u{1b}[48;2;68;195;254;38;2;226;32;254m.\u{1b}[0m",
                )),
            },
            Example {
                description: "draw text in a gradient by specifying foreground start color - end color is assumed to be black",
                example: "'Hello, Nushell! This is a gradient.' | ansi gradient --fgstart '0x40c9ff'",
                result: Some(Value::test_string(
                    "\u{1b}[38;2;64;201;255mH\u{1b}[38;2;62;195;247me\u{1b}[38;2;60;189;240ml\u{1b}[38;2;58;183;233ml\u{1b}[38;2;56;178;225mo\u{1b}[38;2;54;172;218m,\u{1b}[38;2;53;166;211m \u{1b}[38;2;51;160;204mN\u{1b}[38;2;49;155;196mu\u{1b}[38;2;47;149;189ms\u{1b}[38;2;45;143;182mh\u{1b}[38;2;43;137;174me\u{1b}[38;2;42;132;167ml\u{1b}[38;2;40;126;160ml\u{1b}[38;2;38;120;153m!\u{1b}[38;2;36;114;145m \u{1b}[38;2;34;109;138mT\u{1b}[38;2;32;103;131mh\u{1b}[38;2;31;97;123mi\u{1b}[38;2;29;91;116ms\u{1b}[38;2;27;86;109m \u{1b}[38;2;25;80;101mi\u{1b}[38;2;23;74;94ms\u{1b}[38;2;21;68;87m \u{1b}[38;2;20;63;80ma\u{1b}[38;2;18;57;72m \u{1b}[38;2;16;51;65mg\u{1b}[38;2;14;45;58mr\u{1b}[38;2;12;40;50ma\u{1b}[38;2;10;34;43md\u{1b}[38;2;9;28;36mi\u{1b}[38;2;7;22;29me\u{1b}[38;2;5;17;21mn\u{1b}[38;2;3;11;14mt\u{1b}[38;2;1;5;7m.\u{1b}[0m",
                )),
            },
            Example {
                description: "draw text in a gradient by specifying foreground end color - start color is assumed to be black",
                example: "'Hello, Nushell! This is a gradient.' | ansi gradient --fgend '0xe81cff'",
                result: Some(Value::test_string(
                    "\u{1b}[38;2;0;0;0mH\u{1b}[38;2;6;0;7me\u{1b}[38;2;13;1;14ml\u{1b}[38;2;19;2;21ml\u{1b}[38;2;26;3;29mo\u{1b}[38;2;33;4;36m,\u{1b}[38;2;39;4;43m \u{1b}[38;2;46;5;51mN\u{1b}[38;2;53;6;58mu\u{1b}[38;2;59;7;65ms\u{1b}[38;2;66;8;72mh\u{1b}[38;2;72;8;80me\u{1b}[38;2;79;9;87ml\u{1b}[38;2;86;10;94ml\u{1b}[38;2;92;11;102m!\u{1b}[38;2;99;12;109m \u{1b}[38;2;106;12;116mT\u{1b}[38;2;112;13;123mh\u{1b}[38;2;119;14;131mi\u{1b}[38;2;125;15;138ms\u{1b}[38;2;132;16;145m \u{1b}[38;2;139;16;153mi\u{1b}[38;2;145;17;160ms\u{1b}[38;2;152;18;167m \u{1b}[38;2;159;19;174ma\u{1b}[38;2;165;20;182m \u{1b}[38;2;172;20;189mg\u{1b}[38;2;178;21;196mr\u{1b}[38;2;185;22;204ma\u{1b}[38;2;192;23;211md\u{1b}[38;2;198;24;218mi\u{1b}[38;2;205;24;225me\u{1b}[38;2;212;25;233mn\u{1b}[38;2;218;26;240mt\u{1b}[38;2;225;27;247m.\u{1b}[0m",
                )),
            },
            Example {
                description: "draw text in a gradient using a named rainbow foreground gradient",
                example: "'Hello, Nushell! This is a gradient.' | ansi gradient --fgnamed rainbow",
                result: Some(Value::test_string(
                    "\u{1b}[38;2;189;19;84mH\u{1b}[38;2;194;31;76me\u{1b}[38;2;200;43;69ml\u{1b}[38;2;204;56;61ml\u{1b}[38;2;210;69;53mo\u{1b}[38;2;215;82;46m,\u{1b}[38;2;221;94;39m \u{1b}[38;2;228;108;33mN\u{1b}[38;2;227;115;32mu\u{1b}[38;2;226;124;31ms\u{1b}[38;2;226;132;30mh\u{1b}[38;2;226;140;30me\u{1b}[38;2;226;148;29ml\u{1b}[38;2;225;157;28ml\u{1b}[38;2;226;166;29m!\u{1b}[38;2;199;165;30m \u{1b}[38;2;174;164;32mT\u{1b}[38;2;148;163;34mh\u{1b}[38;2;122;164;37mi\u{1b}[38;2;96;163;39ms\u{1b}[38;2;71;162;41m \u{1b}[38;2;46;163;44mi\u{1b}[38;2;46;150;71ms\u{1b}[38;2;47;139;99m \u{1b}[38;2;49;128;127ma\u{1b}[38;2;49;116;154m \u{1b}[38;2;51;105;182mg\u{1b}[38;2;52;94;210mr\u{1b}[38;2;54;83;238ma\u{1b}[38;2;58;75;222md\u{1b}[38;2;62;68;207mi\u{1b}[38;2;67;60;191me\u{1b}[38;2;72;53;175mn\u{1b}[38;2;77;45;160mt\u{1b}[38;2;81;38;145m.\u{1b}[0m",
                )),
            },
            Example {
                description: "draw text in a gradient using a named forest background gradient",
                example: "'Hello, Nushell! This is a gradient.' | ansi gradient --bgnamed forest",
                result: Some(Value::test_string(
                    "\u{1b}[48;2;69;55;48mH\u{1b}[48;2;75;58;49me\u{1b}[48;2;82;63;51ml\u{1b}[48;2;89;68;53ml\u{1b}[48;2;96;71;55mo\u{1b}[48;2;103;76;57m,\u{1b}[48;2;110;81;59m \u{1b}[48;2;117;85;61mN\u{1b}[48;2;123;89;63mu\u{1b}[48;2;127;92;64ms\u{1b}[48;2;117;88;63mh\u{1b}[48;2;107;84;62me\u{1b}[48;2;97;82;61ml\u{1b}[48;2;87;78;60ml\u{1b}[48;2;77;74;59m!\u{1b}[48;2;68;70;58m \u{1b}[48;2;58;67;58mT\u{1b}[48;2;48;63;56mh\u{1b}[48;2;44;63;56mi\u{1b}[48;2;47;66;58ms\u{1b}[48;2;49;69;59m \u{1b}[48;2;52;73;61mi\u{1b}[48;2;54;76;62ms\u{1b}[48;2;56;79;64m \u{1b}[48;2;59;81;65ma\u{1b}[48;2;61;85;67m \u{1b}[48;2;64;88;68mg\u{1b}[48;2;68;91;70mr\u{1b}[48;2;72;93;72ma\u{1b}[48;2;77;96;74md\u{1b}[48;2;83;99;76mi\u{1b}[48;2;87;101;78me\u{1b}[48;2;92;103;80mn\u{1b}[48;2;97;106;82mt\u{1b}[48;2;102;109;84m.\u{1b}[0m",
                )),
            },
        ]
    }
}

// Represents the two supported gradient styles: a named palette or a simple two-color transition.
enum GradientSpec<'a> {
    Named(&'a [Rgb]),
    TwoColor { start: Rgb, end: Rgb },
}

// Shared gradient parameters for foreground and background rendering.
struct GradientOptions<'a> {
    fg_start: Option<Rgb>,
    fg_end: Option<Rgb>,
    fg_palette: Option<&'a [Rgb]>,
    bg_start: Option<Rgb>,
    bg_end: Option<Rgb>,
    bg_palette: Option<&'a [Rgb]>,
}

impl<'a> GradientOptions<'a> {
    fn no_parameters(&self) -> bool {
        self.fg_start.is_none()
            && self.fg_end.is_none()
            && self.bg_start.is_none()
            && self.bg_end.is_none()
            && self.fg_palette.is_none()
            && self.bg_palette.is_none()
    }
}

impl<'a> GradientSpec<'a> {
    fn color_at(&self, index: usize, len: usize) -> Rgb {
        match self {
            GradientSpec::Named(colors) => gradient_color_for_position(colors, index, len),
            GradientSpec::TwoColor { start, end } => {
                gradient_color_for_position(&[*start, *end], index, len)
            }
        }
    }

    fn from_parts(
        start: Option<Rgb>,
        end: Option<Rgb>,
        palette: Option<&'a [Rgb]>,
    ) -> Option<Self> {
        if let Some(colors) = palette {
            Some(GradientSpec::Named(colors))
        } else if start.is_some() || end.is_some() {
            Some(GradientSpec::TwoColor {
                start: start.unwrap_or_else(|| Rgb::new(0, 0, 0)),
                end: end.unwrap_or_else(|| Rgb::new(0, 0, 0)),
            })
        } else {
            None
        }
    }
}

// Parse an optional string flag value into an optional RGB color.
fn value_to_color(v: Option<Value>) -> Result<Option<Rgb>, ShellError> {
    let s = match v {
        None => return Ok(None),
        Some(x) => x.coerce_into_string()?,
    };
    Ok(Some(Rgb::from_hex_string(s)))
}

// Parse a named gradient argument into a static palette reference.
fn value_to_named_gradient(
    v: Option<Value>,
    head: Span,
) -> Result<Option<&'static [Rgb]>, ShellError> {
    let s = match v {
        None => return Ok(None),
        Some(x) => x.coerce_into_string()?,
    };

    let palette = palette_for_name(&s);

    if let Some(palette) = palette {
        Ok(Some(palette))
    } else {
        Err(ShellError::UnsupportedInput {
            msg: format!("Unknown named gradient: '{}'.", s),
            input: "gradient name".into(),
            msg_span: head,
            input_span: head,
        })
    }
}

// Lookup a named gradient palette by case-insensitive name.
fn palette_for_name(name: &str) -> Option<&'static [Rgb]> {
    NAMED_GRADIENTS
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, palette)| *palette)
}

// Build the output text for the --list switch, with sample renderings.
fn named_gradients_list_text(head: Span) -> Value {
    let rows: Vec<Value> = NAMED_GRADIENTS
        .iter()
        .map(|(name, palette)| {
            let sample =
                build_gradient_text("Nushell Gradient", Some(GradientSpec::Named(palette)), None);

            Value::record(
                nu_protocol::record! {
                    "name" => Value::string(name.to_string(), head),
                    "text" => Value::string(sample, head),
                },
                head,
            )
        })
        .collect();

    Value::list(rows, head)
}

// Detect whether the command was invoked with neither input nor any gradient arguments.
fn no_input_and_no_parameters(
    input: &PipelineData,
    options: &GradientOptions<'_>,
    column_paths: &[CellPath],
) -> bool {
    input.is_nothing() && options.no_parameters() && column_paths.is_empty()
}

// Compute the interpolated RGB color for a specific character position in the string.
fn gradient_color_for_position(colors: &[Rgb], index: usize, len: usize) -> Rgb {
    if colors.is_empty() {
        return Rgb::new(0, 0, 0);
    }

    if len <= 1 || colors.len() == 1 {
        return colors[0];
    }

    let t = index as f32 / len as f32;
    let segments = colors.len() - 1;
    let scaled = t * segments as f32;
    let segment_index = scaled.floor().min((segments - 1) as f32) as usize;
    let segment_t = scaled - segment_index as f32;

    colors[segment_index].lerp(colors[segment_index + 1], segment_t)
}

// Render the given string as colored text by applying per-character foreground/background gradients.
fn build_gradient_text(
    value: &str,
    fg_spec: Option<GradientSpec<'_>>,
    bg_spec: Option<GradientSpec<'_>>,
) -> String {
    let len = value.chars().count();
    let mut result = String::with_capacity(value.len().saturating_mul(12));

    for (index, ch) in value.chars().enumerate() {
        let mut style = Style::new();

        if let Some(fg_spec) = &fg_spec {
            let rgb = fg_spec.color_at(index, len);
            style = style.fg(Color::Rgb(rgb.r, rgb.g, rgb.b));
        }

        if let Some(bg_spec) = &bg_spec {
            let rgb = bg_spec.color_at(index, len);
            style = style.on(Color::Rgb(rgb.r, rgb.g, rgb.b));
        }

        write!(&mut result, "{}{}", style.prefix(), ch).expect("writing to string");
    }

    if len > 0 && (fg_spec.is_some() || bg_spec.is_some()) {
        result.push_str("\x1b[0m");
    }

    result
}

fn operate(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let list = call.has_flag(engine_state, stack, "list")?;
    let fgstart: Option<Value> = call.get_flag(engine_state, stack, "fgstart")?;
    let fgend: Option<Value> = call.get_flag(engine_state, stack, "fgend")?;
    let fgnamed: Option<Value> = call.get_flag(engine_state, stack, "fgnamed")?;
    let bgstart: Option<Value> = call.get_flag(engine_state, stack, "bgstart")?;
    let bgend: Option<Value> = call.get_flag(engine_state, stack, "bgend")?;
    let bgnamed: Option<Value> = call.get_flag(engine_state, stack, "bgnamed")?;
    let column_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
    let head = call.head;

    if list {
        return Ok(named_gradients_list_text(head).into_pipeline_data());
    }

    let options = GradientOptions {
        fg_start: value_to_color(fgstart)?,
        fg_end: value_to_color(fgend)?,
        fg_palette: value_to_named_gradient(fgnamed, head)?,
        bg_start: value_to_color(bgstart)?,
        bg_end: value_to_color(bgend)?,
        bg_palette: value_to_named_gradient(bgnamed, head)?,
    };

    if no_input_and_no_parameters(&input, &options, &column_paths) {
        return Err(ShellError::MissingParameter {
            param_name: "please supply input or gradient parameters".into(),
            span: head,
        });
    }

    if options.fg_palette.is_some() && (options.fg_start.is_some() || options.fg_end.is_some()) {
        return Err(ShellError::UnsupportedInput {
            msg: "--gradient-fg cannot be used with --fgstart or --fgend".into(),
            input: "gradient flags".into(),
            msg_span: head,
            input_span: head,
        });
    }

    if options.bg_palette.is_some() && (options.bg_start.is_some() || options.bg_end.is_some()) {
        return Err(ShellError::UnsupportedInput {
            msg: "--gradient-bg cannot be used with --bgstart or --bgend".into(),
            input: "gradient flags".into(),
            msg_span: head,
            input_span: head,
        });
    }

    input.map(
        move |v| {
            if column_paths.is_empty() {
                action(&v, &options, head)
            } else {
                let mut ret = v;
                for path in &column_paths {
                    let options = &options;
                    let r = ret.update_cell_path(
                        &path.members,
                        Box::new(move |old| action(old, options, head)),
                    );
                    if let Err(error) = r {
                        return Value::error(error, head);
                    }
                }
                ret
            }
        },
        engine_state.signals(),
    )
}

fn action(input: &Value, options: &GradientOptions<'_>, command_span: Span) -> Value {
    let span = input.span();
    match input {
        Value::String { val, .. } => {
            let fg_spec =
                GradientSpec::from_parts(options.fg_start, options.fg_end, options.fg_palette);
            let bg_spec =
                GradientSpec::from_parts(options.bg_start, options.bg_end, options.bg_palette);

            if fg_spec.is_none() && bg_spec.is_none() {
                return Value::error(
                    ShellError::MissingParameter {
                        param_name: "please supply foreground and/or background color parameters"
                            .into(),
                        span: command_span,
                    },
                    span,
                );
            }

            let gradient_string = build_gradient_text(val, fg_spec, bg_spec);
            Value::string(gradient_string, span)
        }
        other => {
            let got = format!("value is {}, not string", other.get_type());

            Value::error(
                ShellError::TypeMismatch {
                    err_message: got,
                    span: other.span(),
                },
                other.span(),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GRADIENT_RAINBOW, GradientOptions, SubCommand, action, named_gradients_list_text,
        no_input_and_no_parameters,
    };
    use nu_ansi_term::Rgb;
    use nu_protocol::{PipelineData, Span, Value};

    #[test]
    fn examples_work_as_expected() -> nu_test_support::Result {
        nu_test_support::test().examples(SubCommand)
    }

    #[test]
    fn test_fg_gradient() {
        let input_string = Value::test_string("Hello, World!");
        let expected = Value::test_string(
            "\u{1b}[38;2;64;201;255mH\u{1b}[38;2;76;187;254me\u{1b}[38;2;89;174;254ml\u{1b}[38;2;102;160;254ml\u{1b}[38;2;115;147;254mo\u{1b}[38;2;128;133;254m,\u{1b}[38;2;141;120;254m \u{1b}[38;2;153;107;254mW\u{1b}[38;2;166;94;254mo\u{1b}[38;2;179;80;254mr\u{1b}[38;2;192;67;254ml\u{1b}[38;2;205;53;254md\u{1b}[38;2;218;40;254m!\u{1b}[0m",
        );
        let fg_start = Rgb::from_hex_string("0x40c9ff".to_string());
        let fg_end = Rgb::from_hex_string("0xe81cff".to_string());
        let options = GradientOptions {
            fg_start: Some(fg_start),
            fg_end: Some(fg_end),
            fg_palette: None,
            bg_start: None,
            bg_end: None,
            bg_palette: None,
        };
        let actual = action(&input_string, &options, Span::test_data());
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_named_fg_gradient_rainbow() {
        let input_string = Value::test_string("Hello, Nushell!");
        let options = GradientOptions {
            fg_start: None,
            fg_end: None,
            fg_palette: Some(&GRADIENT_RAINBOW),
            bg_start: None,
            bg_end: None,
            bg_palette: None,
        };
        let actual = action(&input_string, &options, Span::test_data());
        let expected = Value::test_string(
            "\u{1b}[38;2;189;19;84mH\u{1b}[38;2;201;48;66me\u{1b}[38;2;214;78;49ml\u{1b}[38;2;228;108;33ml\u{1b}[38;2;226;126;30mo\u{1b}[38;2;225;145;29m,\u{1b}[38;2;226;166;29m \u{1b}[38;2;165;164;33mN\u{1b}[38;2;105;163;38mu\u{1b}[38;2;46;163;44ms\u{1b}[38;2;48;135;108mh\u{1b}[38;2;51;109;172me\u{1b}[38;2;54;83;238ml\u{1b}[38;2;64;65;201ml\u{1b}[38;2;75;48;166m!\u{1b}[0m",
        );

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_list_named_gradients() {
        let output = named_gradients_list_text(Span::test_data());
        let list = output.as_list().expect("expected list output");

        assert!(list.iter().any(|row| {
            let record = row.as_record().expect("expected row record");
            record
                .get("name")
                .and_then(|value| value.as_str().ok())
                .map(|name: &str| name == "rainbow")
                .unwrap_or(false)
        }));

        assert!(list.iter().any(|row| {
            let record = row.as_record().expect("expected row record");
            record
                .get("name")
                .and_then(|value| value.as_str().ok())
                .map(|name: &str| name == "forest")
                .unwrap_or(false)
        }));

        assert!(list.iter().any(|row| {
            let record = row.as_record().expect("expected row record");
            record
                .get("text")
                .and_then(|value| value.as_str().ok())
                .map(|text: &str| !text.is_empty())
                .unwrap_or(false)
        }));
    }

    #[test]
    fn test_no_parameters_and_no_input_errors() {
        let input = PipelineData::empty();
        let options = GradientOptions {
            fg_start: None,
            fg_end: None,
            fg_palette: None,
            bg_start: None,
            bg_end: None,
            bg_palette: None,
        };
        assert!(no_input_and_no_parameters(&input, &options, &[]));

        let input_nothing = PipelineData::value(Value::nothing(Span::test_data()), None);
        let options = GradientOptions {
            fg_start: None,
            fg_end: None,
            fg_palette: None,
            bg_start: None,
            bg_end: None,
            bg_palette: None,
        };
        assert!(no_input_and_no_parameters(&input_nothing, &options, &[]));
    }
}
