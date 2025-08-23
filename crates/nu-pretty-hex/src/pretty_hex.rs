use core::fmt;
use nu_ansi_term::{Color, Style};

/// Returns a one-line hexdump of `source` grouped in default format without header
/// and ASCII column.
pub fn simple_hex<T: AsRef<[u8]>>(source: &T) -> String {
    let mut writer = String::new();
    hex_write(&mut writer, source, HexConfig::simple(), None).unwrap_or(());
    writer
}

/// Dump `source` as hex octets in default format without header and ASCII column to the `writer`.
pub fn simple_hex_write<T, W>(writer: &mut W, source: &T) -> fmt::Result
where
    T: AsRef<[u8]>,
    W: fmt::Write,
{
    hex_write(writer, source, HexConfig::simple(), None)
}

/// Return a multi-line hexdump in default format complete with addressing, hex digits,
/// and ASCII representation.
pub fn pretty_hex<T: AsRef<[u8]>>(source: &T) -> String {
    let mut writer = String::new();
    hex_write(&mut writer, source, HexConfig::default(), Some(true)).unwrap_or(());
    writer
}

/// Write multi-line hexdump in default format complete with addressing, hex digits,
/// and ASCII representation to the writer.
pub fn pretty_hex_write<T, W>(writer: &mut W, source: &T) -> fmt::Result
where
    T: AsRef<[u8]>,
    W: fmt::Write,
{
    hex_write(writer, source, HexConfig::default(), Some(true))
}

/// Return a hexdump of `source` in specified format.
pub fn config_hex<T: AsRef<[u8]>>(source: &T, cfg: HexConfig) -> String {
    let mut writer = String::new();
    hex_write(&mut writer, source, cfg, Some(true)).unwrap_or(());
    writer
}

/// Configuration parameters for hexdump.
#[derive(Clone, Copy, Debug)]
pub struct HexConfig {
    /// Write first line header with data length.
    pub title: bool,
    /// Append ASCII representation column.
    pub ascii: bool,
    /// Source bytes per row. 0 for single row without address prefix.
    pub width: usize,
    /// Chunks count per group. 0 for single group (column).
    pub group: usize,
    /// Source bytes per chunk (word). 0 for single word.
    pub chunk: usize,
    /// Offset to start counting addresses from
    pub address_offset: usize,
    /// Bytes from 0 to skip
    pub skip: Option<usize>,
    /// Length to return
    pub length: Option<usize>,
}

/// Default configuration with `title`, `ascii`, 16 source bytes `width` grouped to 4 separate
/// hex bytes. Using in `pretty_hex`, `pretty_hex_write` and `fmt::Debug` implementation.
impl Default for HexConfig {
    fn default() -> HexConfig {
        HexConfig {
            title: true,
            ascii: true,
            width: 16,
            group: 4,
            chunk: 1,
            address_offset: 0,
            skip: None,
            length: None,
        }
    }
}

impl HexConfig {
    /// Returns configuration for `simple_hex`, `simple_hex_write` and `fmt::Display` implementation.
    pub fn simple() -> Self {
        HexConfig::default().to_simple()
    }

    fn delimiter(&self, i: usize) -> &'static str {
        if i > 0 && self.chunk > 0 && i.is_multiple_of(self.chunk) {
            if self.group > 0 && i.is_multiple_of(self.group * self.chunk) {
                "  "
            } else {
                " "
            }
        } else {
            ""
        }
    }

    fn to_simple(self) -> Self {
        HexConfig {
            title: false,
            ascii: false,
            width: 0,
            ..self
        }
    }
}

pub fn categorize_byte(byte: &u8) -> (Style, Option<char>) {
    // This section is here so later we can configure these items
    let null_char_style = Style::default().fg(Color::Fixed(242));
    let null_char = Some('0');
    let ascii_printable_style = Style::default().fg(Color::Cyan).bold();
    let ascii_printable = None;
    let ascii_space_style = Style::default().fg(Color::Green).bold();
    let ascii_space = Some(' ');
    let ascii_white_space_style = Style::default().fg(Color::Green).bold();
    let ascii_white_space = Some('_');
    let ascii_other_style = Style::default().fg(Color::Purple).bold();
    let ascii_other = Some('•');
    let non_ascii_style = Style::default().fg(Color::Yellow).bold();
    let non_ascii = Some('×'); // or Some('.')

    if byte == &0 {
        (null_char_style, null_char)
    } else if byte.is_ascii_graphic() {
        (ascii_printable_style, ascii_printable)
    } else if byte.is_ascii_whitespace() {
        // 0x20 == 32 decimal - replace with a real space
        if byte == &32 {
            (ascii_space_style, ascii_space)
        } else {
            (ascii_white_space_style, ascii_white_space)
        }
    } else if byte.is_ascii() {
        (ascii_other_style, ascii_other)
    } else {
        (non_ascii_style, non_ascii)
    }
}

/// Write hex dump in specified format.
pub fn hex_write<T, W>(
    writer: &mut W,
    source: &T,
    cfg: HexConfig,
    with_color: Option<bool>,
) -> fmt::Result
where
    T: AsRef<[u8]>,
    W: fmt::Write,
{
    let use_color = with_color.unwrap_or(false);

    if source.as_ref().is_empty() {
        return Ok(());
    }

    let amount = cfg.length.unwrap_or_else(|| source.as_ref().len());

    let skip = cfg.skip.unwrap_or(0);

    let address_offset = cfg.address_offset;

    let source_part_vec: Vec<u8> = source
        .as_ref()
        .iter()
        .skip(skip)
        .take(amount)
        .copied()
        .collect();

    if cfg.title {
        write_title(
            writer,
            HexConfig {
                length: Some(source_part_vec.len()),
                ..cfg
            },
            use_color,
        )?;
    }

    let lines = source_part_vec.chunks(if cfg.width > 0 {
        cfg.width
    } else {
        source_part_vec.len()
    });

    let lines_len = lines.len();

    for (i, row) in lines.enumerate() {
        if cfg.width > 0 {
            let style = Style::default().fg(Color::Cyan);
            if use_color {
                write!(
                    writer,
                    "{}{:08x}{}:   ",
                    style.prefix(),
                    i * cfg.width + skip + address_offset,
                    style.suffix()
                )?;
            } else {
                write!(writer, "{:08x}:   ", i * cfg.width + skip + address_offset,)?;
            }
        }
        for (i, x) in row.as_ref().iter().enumerate() {
            if use_color {
                let (style, _char) = categorize_byte(x);
                write!(
                    writer,
                    "{}{}{:02x}{}",
                    cfg.delimiter(i),
                    style.prefix(),
                    x,
                    style.suffix()
                )?;
            } else {
                write!(writer, "{}{:02x}", cfg.delimiter(i), x,)?;
            }
        }
        if cfg.ascii {
            for j in row.len()..cfg.width {
                write!(writer, "{}  ", cfg.delimiter(j))?;
            }
            write!(writer, "   ")?;
            for x in row {
                let (style, a_char) = categorize_byte(x);
                let replacement_char = a_char.unwrap_or(*x as char);
                if use_color {
                    write!(
                        writer,
                        "{}{}{}",
                        style.prefix(),
                        replacement_char,
                        style.suffix()
                    )?;
                } else {
                    write!(writer, "{replacement_char}",)?;
                }
            }
        }
        if i + 1 < lines_len {
            writeln!(writer)?;
        }
    }
    Ok(())
}

/// Write the title for the given config. The length will be taken from `cfg.length`.
pub fn write_title<W>(writer: &mut W, cfg: HexConfig, use_color: bool) -> Result<(), fmt::Error>
where
    W: fmt::Write,
{
    let write = |writer: &mut W, length: fmt::Arguments<'_>| {
        if use_color {
            writeln!(
                writer,
                "Length: {length} | {0}printable {1}whitespace {2}ascii_other {3}non_ascii{4}",
                Style::default().fg(Color::Cyan).bold().prefix(),
                Style::default().fg(Color::Green).bold().prefix(),
                Style::default().fg(Color::Purple).bold().prefix(),
                Style::default().fg(Color::Yellow).bold().prefix(),
                Style::default().fg(Color::Yellow).suffix()
            )
        } else {
            writeln!(writer, "Length: {length}")
        }
    };

    if let Some(len) = cfg.length {
        write(writer, format_args!("{len} (0x{len:x}) bytes"))
    } else {
        write(writer, format_args!("unknown (stream)"))
    }
}

/// Reference wrapper for use in arguments formatting.
pub struct Hex<'a, T: 'a>(&'a T, HexConfig);

impl<'a, T: 'a + AsRef<[u8]>> fmt::Display for Hex<'a, T> {
    /// Formats the value by `simple_hex_write` using the given formatter.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        hex_write(f, self.0, self.1.to_simple(), None)
    }
}

impl<'a, T: 'a + AsRef<[u8]>> fmt::Debug for Hex<'a, T> {
    /// Formats the value by `pretty_hex_write` using the given formatter.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        hex_write(f, self.0, self.1, None)
    }
}

/// Allows generates hex dumps to a formatter.
pub trait PrettyHex: Sized {
    /// Wrap self reference for use in `std::fmt::Display` and `std::fmt::Debug`
    /// formatting as hex dumps.
    fn hex_dump(&self) -> Hex<Self>;

    /// Wrap self reference for use in `std::fmt::Display` and `std::fmt::Debug`
    /// formatting as hex dumps in specified format.
    fn hex_conf(&self, cfg: HexConfig) -> Hex<Self>;
}

impl<T> PrettyHex for T
where
    T: AsRef<[u8]>,
{
    fn hex_dump(&self) -> Hex<Self> {
        Hex(self, HexConfig::default())
    }
    fn hex_conf(&self, cfg: HexConfig) -> Hex<Self> {
        Hex(self, cfg)
    }
}
