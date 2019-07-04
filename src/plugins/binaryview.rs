use crossterm::{cursor, terminal, Attribute, Color, Colored, RawScreen};
use indexmap::IndexMap;
use nu::{serve_plugin, Args, CommandConfig, Plugin, ShellError, Value};

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
            mandatory_positional: vec![],
            optional_positional: vec![],
            can_load: vec![],
            can_save: vec![],
            is_filter: false,
            is_sink: true,
            named: IndexMap::new(),
            rest_positional: true,
        })
    }

    fn sink(&mut self, _args: Args, input: Vec<Value>) {
        for v in input {
            match v {
                Value::Binary(b) => {
                    let _ = view_binary(&b);
                }
                _ => {}
            }
        }
    }
}

fn view_binary(b: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    use pretty_hex::*;
    if b.len() > 3 {
        match (b[0], b[1], b[2]) {
            (0x4e, 0x45, 0x53) => {
                view_contents(b)?;
                return Ok(());
            }
            _ => {}
        }
    }
    println!("{:?}", b.hex_dump());
    Ok(())
}

#[derive(PartialEq, Debug)]
pub enum JoyButton {
    A = 0,
    B = 1,
    Select = 2,
    Start = 3,
    Up = 4,
    Down = 5,
    Left = 6,
    Right = 7,
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

        let mut prev_color = None;

        for pixel in &self.frame_buffer {
            match prev_color {
                Some(c) if c == pixel.1 => {
                    print!("{}", pixel.0);
                }
                _ => {
                    prev_color = Some(pixel.1);
                    print!(
                        "{}{}{}",
                        Colored::Fg(Color::Rgb {
                            r: (pixel.1).0,
                            g: (pixel.1).1,
                            b: (pixel.1).2
                        }),
                        Colored::Bg(Color::Rgb {
                            r: 25,
                            g: 25,
                            b: 25
                        }),
                        pixel.0
                    )
                }
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

pub fn view_contents(buffer: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
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

            let mut new_offscreen = vec![0; context.height * context.width * 4];
            let mut resizer = resize::new(
                256,
                240,
                context.width,
                context.height,
                resize::Pixel::RGBA,
                resize::Type::Triangle,
            );
            let slice = unsafe { std::slice::from_raw_parts(image_buffer, 256 * 240 * 4) };
            resizer.resize(&slice, &mut new_offscreen);

            context.clear();

            for row in 0..context.height {
                for col in 0..(context.width) {
                    let red = new_offscreen[col * 4 + row * context.width * 4];
                    let green = new_offscreen[col * 4 + 1 + row * context.width * 4];
                    let blue = new_offscreen[col * 4 + 2 + row * context.width * 4];

                    context.frame_buffer[col + row * context.width] = ('@', (red, green, blue));
                }
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
