extern crate nu_ansi_term;
use nu_ansi_term::{Color::*, Style};

// This example prints out the 16 basic colors.

fn main() {
    let normal = Style::default();

    println!("{} {}", normal.paint("Normal"), normal.bold().paint("bold"));
    println!("{} {}", Black.paint("Black"), Black.bold().paint("bold"));
    println!("{} {}", Red.paint("Red"), Red.bold().paint("bold"));
    println!("{} {}", Green.paint("Green"), Green.bold().paint("bold"));
    println!("{} {}", Yellow.paint("Yellow"), Yellow.bold().paint("bold"));
    println!("{} {}", Blue.paint("Blue"), Blue.bold().paint("bold"));
    println!("{} {}", Purple.paint("Purple"), Purple.bold().paint("bold"));
    println!("{} {}", Cyan.paint("Cyan"), Cyan.bold().paint("bold"));
    println!("{} {}", White.paint("White"), White.bold().paint("bold"));
}
