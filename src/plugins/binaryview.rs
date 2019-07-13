use crossterm::{cursor, terminal, Attribute, RawScreen};
use indexmap::IndexMap;
use nu::{serve_plugin, Args, CommandConfig, Plugin, ShellError, Spanned, Value};
use pretty_hex::*;

struct BinaryView;

impl BinaryView {
    fn new() -> BinaryView {
        BinaryView
    }
}

impl Plugin for BinaryView {
    fn config(&mut self) -> Result<CommandConfig, ShellError> {
        Ok(CommandConfig {
            name: "binaryview".to_string(),
            positional: vec![],
            is_filter: false,
            is_sink: true,
            named: IndexMap::new(),
            rest_positional: true,
        })
    }

    fn sink(&mut self, _args: Args, input: Vec<Spanned<Value>>) {
        for v in input {
            match v {
                Spanned {
                    item: Value::Binary(b),
                    ..
                } => {
                    let _ = view_binary(&b);
                }
                _ => {}
            }
        }
    }
}

fn view_binary(b: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    if b.len() > 3 {
        match (b[0], b[1], b[2]) {
            (0x4e, 0x45, 0x53) => {
                view_contents_interactive(b)?;
                return Ok(());
            }
            _ => {}
        }
    }
    view_contents(b)?;
    Ok(())
}

pub struct Context {
    pub width: usize,
    pub height: usize,
    pub frame_buffer: Vec<(char, (u8, u8, u8))>,
    pub since_last_button: Vec<usize>,
}

impl Context {
    pub fn blank() -> Context {
        Context {
            width: 0,
            height: 0,
            frame_buffer: vec![],
            since_last_button: vec![0; 8],
        }
    }
    pub fn clear(&mut self) {
        self.frame_buffer = vec![(' ', (0, 0, 0)); self.width * self.height as usize];
    }
    pub fn flush(&self) -> Result<(), Box<dyn std::error::Error>> {
        let cursor = cursor();
        cursor.goto(0, 0)?;

        let mut prev_color: Option<(u8, u8, u8)> = None;
        let mut prev_count = 1;

        for pixel in &self.frame_buffer {
            match prev_color {
                Some(c) if c == pixel.1 => {
                    prev_count += 1;
                }
                Some(c) => {
                    print!(
                        "{}",
                        ansi_term::Colour::RGB(c.0, c.1, c.2)
                            .paint((0..prev_count).map(|_| pixel.0).collect::<String>())
                    );
                    prev_color = Some(pixel.1);
                    prev_count = 1;
                }
                _ => {
                    prev_color = Some(pixel.1);
                    prev_count = 1;
                }
            }
        }

        if prev_count > 0 {
            if let Some(color) = prev_color {
                print!(
                    "{}",
                    ansi_term::Colour::RGB(color.0, color.1, color.2)
                        .paint((0..prev_count).map(|_| "@").collect::<String>())
                );
            }
        }

        println!("{}", Attribute::Reset);

        Ok(())
    }
    pub fn update(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let terminal = terminal();
        let terminal_size = terminal.terminal_size();

        if (self.width != terminal_size.0 as usize) || (self.height != terminal_size.1 as usize) {
            let cursor = cursor();
            cursor.hide()?;

            self.width = terminal_size.0 as usize + 1;
            self.height = terminal_size.1 as usize;
        }

        Ok(())
    }
}

#[derive(Debug)]
struct RawImageBuffer {
    dimensions: (u64, u64),
    colortype: image::ColorType,
    buffer: Vec<u8>,
}

fn load_from_png_buffer(buffer: &[u8]) -> Option<(RawImageBuffer)> {
    use image::ImageDecoder;

    let decoder = image::png::PNGDecoder::new(buffer);
    if decoder.is_err() {
        return None;
    }
    let decoder = decoder.unwrap();

    let dimensions = decoder.dimensions();
    let colortype = decoder.colortype();
    let buffer = decoder.read_image().unwrap();

    Some(RawImageBuffer {
        dimensions,
        colortype,
        buffer,
    })
}

fn load_from_jpg_buffer(buffer: &[u8]) -> Option<(RawImageBuffer)> {
    use image::ImageDecoder;

    let decoder = image::jpeg::JPEGDecoder::new(buffer);
    if decoder.is_err() {
        return None;
    }
    let decoder = decoder.unwrap();

    let dimensions = decoder.dimensions();
    let colortype = decoder.colortype();
    let buffer = decoder.read_image().unwrap();

    Some(RawImageBuffer {
        dimensions,
        colortype,
        buffer,
    })
}

pub fn view_contents(buffer: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let mut raw_image_buffer = load_from_png_buffer(buffer);

    if raw_image_buffer.is_none() {
        raw_image_buffer = load_from_jpg_buffer(buffer);
    }

    if raw_image_buffer.is_none() {
        //Not yet supported
        println!("{:?}", buffer.hex_dump());
        return Ok(());
    }
    let raw_image_buffer = raw_image_buffer.unwrap();

    let mut context: Context = Context::blank();
    let _ = context.update();
    context.clear();

    match raw_image_buffer.colortype {
        image::ColorType::RGBA(8) => {
            let img = image::ImageBuffer::<image::Rgba<u8>, Vec<u8>>::from_vec(
                raw_image_buffer.dimensions.0 as u32,
                raw_image_buffer.dimensions.1 as u32,
                raw_image_buffer.buffer,
            )
            .unwrap();

            let resized_img = image::imageops::resize(
                &img,
                context.width as u32,
                context.height as u32,
                image::FilterType::Lanczos3,
            );

            let mut count = 0;
            for pixel in resized_img.pixels() {
                use image::Pixel;
                let rgb = pixel.to_rgb();
                //print!("{}", rgb[0]);
                context.frame_buffer[count] = ('@', (rgb[0], rgb[1], rgb[2]));
                count += 1;
            }
        }
        image::ColorType::RGB(8) => {
            let img = image::ImageBuffer::<image::Rgb<u8>, Vec<u8>>::from_vec(
                raw_image_buffer.dimensions.0 as u32,
                raw_image_buffer.dimensions.1 as u32,
                raw_image_buffer.buffer,
            )
            .unwrap();

            let resized_img = image::imageops::resize(
                &img,
                context.width as u32,
                context.height as u32,
                image::FilterType::Lanczos3,
            );

            let mut count = 0;
            for pixel in resized_img.pixels() {
                use image::Pixel;
                let rgb = pixel.to_rgb();
                //print!("{}", rgb[0]);
                context.frame_buffer[count] = ('@', (rgb[0], rgb[1], rgb[2]));
                count += 1;
            }
        }
        _ => {
            //Not yet supported
            println!("{:?}", buffer.hex_dump());
            return Ok(());
        }
    }

    context.flush()?;

    let cursor = cursor();
    let _ = cursor.show();

    #[allow(unused)]
    let screen = RawScreen::disable_raw_mode();

    Ok(())
}

pub fn view_contents_interactive(buffer: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    use rawkey::{KeyCode, RawKey};

    let mut nes = neso::Nes::new(48000.0);
    let rawkey = RawKey::new();
    nes.load_rom(&buffer);

    nes.reset();

    if let Ok(_raw) = RawScreen::into_raw_mode() {
        let mut context: Context = Context::blank();
        let input = crossterm::input();
        let _ = input.read_async();
        let cursor = cursor();

        let buttons = vec![
            KeyCode::LShift,
            KeyCode::LControl,
            KeyCode::Tab,
            KeyCode::Back,
            KeyCode::UpArrow,
            KeyCode::DownArrow,
            KeyCode::LeftArrow,
            KeyCode::RightArrow,
        ];

        cursor.hide()?;

        'gameloop: loop {
            let _ = context.update();
            nes.step_frame();

            let image_buffer = nes.image_buffer();

            let slice = unsafe { std::slice::from_raw_parts(image_buffer, 256 * 240 * 4) };
            let img =
                image::ImageBuffer::<image::Rgba<u8>, &[u8]>::from_raw(256, 240, slice).unwrap();
            let resized_img = image::imageops::resize(
                &img,
                context.width as u32,
                context.height as u32,
                image::FilterType::Lanczos3,
            );

            context.clear();

            let mut count = 0;
            for pixel in resized_img.pixels() {
                use image::Pixel;
                let rgb = pixel.to_rgb();

                context.frame_buffer[count] = ('@', (rgb[0], rgb[1], rgb[2]));
                count += 1;
            }
            context.flush()?;

            if rawkey.is_pressed(rawkey::KeyCode::Escape) {
                break 'gameloop;
            } else {
                for i in 0..buttons.len() {
                    if rawkey.is_pressed(buttons[i]) {
                        nes.press_button(0, i as u8);
                    } else {
                        nes.release_button(0, i as u8);
                    }
                }
            }
        }
    }

    let cursor = cursor();
    let _ = cursor.show();

    #[allow(unused)]
    let screen = RawScreen::disable_raw_mode();

    Ok(())
}

fn main() {
    serve_plugin(&mut BinaryView::new());
}
