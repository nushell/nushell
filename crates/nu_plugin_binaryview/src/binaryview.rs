use crossterm::{style::Attribute, ExecutableCommand};
use nu_protocol::outln;
use nu_source::AnchorLocation;
use pretty_hex::*;

#[derive(Default)]
pub struct BinaryView;

impl BinaryView {
    pub fn new() -> BinaryView {
        BinaryView
    }
}

pub fn view_binary(
    b: &[u8],
    source: Option<&AnchorLocation>,
    lores_mode: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if b.len() > 3 {
        if let (0x4e, 0x45, 0x53) = (b[0], b[1], b[2]) {
            view_contents_interactive(b, source, lores_mode)?;
            return Ok(());
        }
    }
    view_contents(b, source, lores_mode)?;
    Ok(())
}

pub struct RenderContext {
    pub width: usize,
    pub height: usize,
    pub frame_buffer: Vec<(u8, u8, u8)>,
    pub since_last_button: Vec<usize>,
    pub lores_mode: bool,
}

impl RenderContext {
    pub fn blank(lores_mode: bool) -> RenderContext {
        RenderContext {
            width: 0,
            height: 0,
            frame_buffer: vec![],
            since_last_button: vec![0; 8],
            lores_mode,
        }
    }
    pub fn clear(&mut self) {
        self.frame_buffer = vec![(0, 0, 0); self.width * self.height as usize];
    }

    fn render_to_screen_lores(&mut self) {
        let mut prev_color: Option<(u8, u8, u8)> = None;
        let mut prev_count = 1;

        let _ = std::io::stdout().execute(crossterm::cursor::MoveTo(0, 0));

        for pixel in &self.frame_buffer {
            match prev_color {
                Some(c) if c == *pixel => {
                    prev_count += 1;
                }
                Some(c) => {
                    print!(
                        "{}",
                        nu_ansi_term::Color::RGB(c.0, c.1, c.2)
                            .paint((0..prev_count).map(|_| "█").collect::<String>())
                    );
                    prev_color = Some(*pixel);
                    prev_count = 1;
                }
                _ => {
                    prev_color = Some(*pixel);
                    prev_count = 1;
                }
            }
        }

        if prev_count > 0 {
            if let Some(color) = prev_color {
                print!(
                    "{}",
                    nu_ansi_term::Color::RGB(color.0, color.1, color.2)
                        .paint((0..prev_count).map(|_| "█").collect::<String>())
                );
            }
        }
        outln!("{}", Attribute::Reset);
    }
    fn render_to_screen_hires(&mut self) {
        let mut prev_fg: Option<(u8, u8, u8)> = None;
        let mut prev_bg: Option<(u8, u8, u8)> = None;
        let mut prev_count = 1;

        let mut pos = 0;
        let fb_len = self.frame_buffer.len();

        let _ = std::io::stdout().execute(crossterm::cursor::MoveTo(0, 0));

        while pos < (fb_len - self.width) {
            let top_pixel = self.frame_buffer[pos];
            let bottom_pixel = self.frame_buffer[pos + self.width];

            match (prev_fg, prev_bg) {
                (Some(c), Some(d)) if c == top_pixel && d == bottom_pixel => {
                    prev_count += 1;
                }
                (Some(c), Some(d)) => {
                    print!(
                        "{}",
                        nu_ansi_term::Color::RGB(c.0, c.1, c.2)
                            .on(nu_ansi_term::Color::RGB(d.0, d.1, d.2,))
                            .paint((0..prev_count).map(|_| "▀").collect::<String>())
                    );
                    prev_fg = Some(top_pixel);
                    prev_bg = Some(bottom_pixel);
                    prev_count = 1;
                }
                _ => {
                    prev_fg = Some(top_pixel);
                    prev_bg = Some(bottom_pixel);
                    prev_count = 1;
                }
            }
            pos += 1;
            if pos % self.width == 0 {
                pos += self.width;
            }
        }
        if prev_count > 0 {
            if let (Some(c), Some(d)) = (prev_fg, prev_bg) {
                print!(
                    "{}",
                    nu_ansi_term::Color::RGB(c.0, c.1, c.2)
                        .on(nu_ansi_term::Color::RGB(d.0, d.1, d.2,))
                        .paint((0..prev_count).map(|_| "▀").collect::<String>())
                );
            }
        }
        outln!("{}", Attribute::Reset);
    }
    pub fn flush(&mut self) {
        if self.lores_mode {
            self.render_to_screen_lores()
        } else {
            self.render_to_screen_hires()
        }
    }
    pub fn update(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let terminal_size = crossterm::terminal::size().unwrap_or((80, 24));

        if (self.width != terminal_size.0 as usize) || (self.height != terminal_size.1 as usize) {
            let _ = std::io::stdout().execute(crossterm::cursor::Hide);

            self.width = terminal_size.0 as usize;
            self.height = if self.lores_mode {
                terminal_size.1 as usize - 1
            } else {
                (terminal_size.1 as usize - 1) * 2
            };
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

fn load_from_png_buffer(buffer: &[u8]) -> Result<RawImageBuffer, Box<dyn std::error::Error>> {
    use image::ImageDecoder;

    let decoder = image::png::PNGDecoder::new(buffer)?;

    let dimensions = decoder.dimensions();
    let colortype = decoder.colortype();
    let buffer = decoder.read_image()?;

    Ok(RawImageBuffer {
        dimensions,
        colortype,
        buffer,
    })
}

fn load_from_jpg_buffer(buffer: &[u8]) -> Result<RawImageBuffer, Box<dyn std::error::Error>> {
    use image::ImageDecoder;

    let decoder = image::jpeg::JPEGDecoder::new(buffer)?;

    let dimensions = decoder.dimensions();
    let colortype = decoder.colortype();
    let buffer = decoder.read_image()?;

    Ok(RawImageBuffer {
        dimensions,
        colortype,
        buffer,
    })
}

pub fn view_contents(
    buffer: &[u8],
    _source: Option<&AnchorLocation>,
    lores_mode: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut raw_image_buffer = load_from_png_buffer(buffer);

    if raw_image_buffer.is_err() {
        raw_image_buffer = load_from_jpg_buffer(buffer);
    }

    if raw_image_buffer.is_err() {
        //Not yet supported
        outln!("{:?}", buffer.hex_dump());
        return Ok(());
    }
    let raw_image_buffer = raw_image_buffer?;

    let mut render_context: RenderContext = RenderContext::blank(lores_mode);
    let _ = render_context.update();
    render_context.clear();

    match raw_image_buffer.colortype {
        image::ColorType::RGBA(8) => {
            let img = image::ImageBuffer::<image::Rgba<u8>, Vec<u8>>::from_vec(
                raw_image_buffer.dimensions.0 as u32,
                raw_image_buffer.dimensions.1 as u32,
                raw_image_buffer.buffer,
            )
            .ok_or("Cannot convert image data")?;

            let resized_img = image::imageops::resize(
                &img,
                render_context.width as u32,
                render_context.height as u32,
                image::FilterType::Lanczos3,
            );

            for (count, pixel) in resized_img.pixels().enumerate() {
                use image::Pixel;
                let rgb = pixel.to_rgb();
                render_context.frame_buffer[count] = (rgb[0], rgb[1], rgb[2]);
            }
        }
        image::ColorType::RGB(8) => {
            let img = image::ImageBuffer::<image::Rgb<u8>, Vec<u8>>::from_vec(
                raw_image_buffer.dimensions.0 as u32,
                raw_image_buffer.dimensions.1 as u32,
                raw_image_buffer.buffer,
            )
            .ok_or("Cannot convert image data")?;

            let resized_img = image::imageops::resize(
                &img,
                render_context.width as u32,
                render_context.height as u32,
                image::FilterType::Lanczos3,
            );

            for (count, pixel) in resized_img.pixels().enumerate() {
                use image::Pixel;
                let rgb = pixel.to_rgb();
                render_context.frame_buffer[count] = (rgb[0], rgb[1], rgb[2]);
            }
        }
        _ => {
            //Not yet supported
            outln!("{:?}", buffer.hex_dump());
            return Ok(());
        }
    }

    render_context.flush();

    let _ = std::io::stdout().execute(crossterm::cursor::Show);

    let _ = crossterm::terminal::disable_raw_mode();

    Ok(())
}

pub fn view_contents_interactive(
    buffer: &[u8],
    source: Option<&AnchorLocation>,
    lores_mode: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use rawkey::{KeyCode, RawKey};

    let sav_path = if let Some(AnchorLocation::File(f)) = source {
        let mut path = std::path::PathBuf::from(f);
        path.set_extension("sav");
        Some(path)
    } else {
        None
    };

    let mut nes = neso::Nes::new(0.0);
    let rawkey = RawKey::new();
    nes.load_rom(&buffer);

    if let Some(ref sav_path) = sav_path {
        if let Ok(contents) = std::fs::read(sav_path) {
            let _ = nes.load_state(&contents);
        }
    }

    nes.reset();

    if let Ok(_raw) = crossterm::terminal::enable_raw_mode() {
        let mut render_context: RenderContext = RenderContext::blank(lores_mode);

        let buttons = vec![
            KeyCode::Alt,
            KeyCode::LeftControl,
            KeyCode::Tab,
            KeyCode::BackSpace,
            KeyCode::UpArrow,
            KeyCode::DownArrow,
            KeyCode::LeftArrow,
            KeyCode::RightArrow,
        ];

        let _ = std::io::stdout().execute(crossterm::cursor::Hide);

        'gameloop: loop {
            let _ = render_context.update();
            nes.step_frame();

            let image_buffer = nes.image_buffer();

            let slice = unsafe { std::slice::from_raw_parts(image_buffer, 256 * 240 * 4) };
            let img = image::ImageBuffer::<image::Rgba<u8>, &[u8]>::from_raw(256, 240, slice)
                .ok_or("Cannot convert image data")?;
            let resized_img = image::imageops::resize(
                &img,
                render_context.width as u32,
                render_context.height as u32,
                image::FilterType::Lanczos3,
            );

            render_context.clear();

            for (count, pixel) in resized_img.pixels().enumerate() {
                use image::Pixel;
                let rgb = pixel.to_rgb();

                render_context.frame_buffer[count] = (rgb[0], rgb[1], rgb[2]);
            }
            render_context.flush();

            if rawkey.is_pressed(rawkey::KeyCode::Escape) {
                break 'gameloop;
            } else {
                for (idx, button) in buttons.iter().enumerate() {
                    if rawkey.is_pressed(*button) {
                        nes.press_button(0, idx as u8);
                    } else {
                        nes.release_button(0, idx as u8);
                    }
                }
                loop {
                    let x = crossterm::event::poll(std::time::Duration::from_secs(0));
                    match x {
                        Ok(true) => {
                            // Swallow the events so we don't queue them into the line editor
                            let _ = crossterm::event::read();
                        }
                        _ => {
                            break;
                        }
                    }
                }
            }
        }
    }

    if let Some(ref sav_path) = sav_path {
        let buffer = nes.save_state();
        if let Ok(buffer) = buffer {
            let _ = std::fs::write(sav_path, buffer);
        }
    }

    let _ = std::io::stdout().execute(crossterm::cursor::Show);

    let _screen = crossterm::terminal::disable_raw_mode();

    Ok(())
}
