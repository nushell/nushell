// Code liberally borrowed from here
// https://github.com/navierr/coloriz
use std::ops;
use std::u32;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    /// Red
    pub r: u8,
    /// Green
    pub g: u8,
    /// Blue
    pub b: u8,
}

impl Rgb {
    /// Creates a new [Rgb] color
    #[inline]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Creates a new [Rgb] color with a hex code
    #[inline]
    pub const fn from_hex(hex: u32) -> Self {
        Self::new((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
    }

    pub fn from_hex_string(hex: String) -> Self {
        if hex.chars().count() == 8 && hex.starts_with("0x") {
            // eprintln!("hex:{:?}", hex);
            let (_, value_string) = hex.split_at(2);
            // eprintln!("value_string:{:?}", value_string);
            let int_val = u64::from_str_radix(value_string, 16);
            match int_val {
                Ok(num) => Self::new(
                    ((num & 0xff0000) >> 16) as u8,
                    ((num & 0xff00) >> 8) as u8,
                    (num & 0xff) as u8,
                ),
                // Don't fail, just make the color black
                // Should we fail?
                _ => Self::new(0, 0, 0),
            }
        } else {
            // Don't fail, just make the color black.
            // Should we fail?
            Self::new(0, 0, 0)
        }
    }

    /// Creates a new [Rgb] color with three [f32] values
    pub fn from_f32(r: f32, g: f32, b: f32) -> Self {
        Self::new(
            (r.clamp(0.0, 1.0) * 255.0) as u8,
            (g.clamp(0.0, 1.0) * 255.0) as u8,
            (b.clamp(0.0, 1.0) * 255.0) as u8,
        )
    }

    /// Creates a grayscale [Rgb] color
    #[inline]
    pub const fn gray(x: u8) -> Self {
        Self::new(x, x, x)
    }

    /// Creates a grayscale [Rgb] color with a [f32] value
    pub fn gray_f32(x: f32) -> Self {
        Self::from_f32(x, x, x)
    }

    /// Creates a new [Rgb] color from a [HSL] color
    // pub fn from_hsl(hsl: HSL) -> Self {
    //     if hsl.s == 0.0 {
    //         return Self::gray_f32(hsl.l);
    //     }

    //     let q = if hsl.l < 0.5 {
    //         hsl.l * (1.0 + hsl.s)
    //     } else {
    //         hsl.l + hsl.s - hsl.l * hsl.s
    //     };
    //     let p = 2.0 * hsl.l - q;
    //     let h2c = |t: f32| {
    //         let t = t.clamp(0.0, 1.0);
    //         if 6.0 * t < 1.0 {
    //             p + 6.0 * (q - p) * t
    //         } else if t < 0.5 {
    //             q
    //         } else if 1.0 < 1.5 * t {
    //             p + 6.0 * (q - p) * (1.0 / 1.5 - t)
    //         } else {
    //             p
    //         }
    //     };

    //     Self::from_f32(h2c(hsl.h + 1.0 / 3.0), h2c(hsl.h), h2c(hsl.h - 1.0 / 3.0))
    // }

    /// Computes the linear interpolation between `self` and `other` for `t`
    pub fn lerp(&self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        self * (1.0 - t) + other * t
    }
}

impl From<(u8, u8, u8)> for Rgb {
    fn from((r, g, b): (u8, u8, u8)) -> Self {
        Self::new(r, g, b)
    }
}

impl From<(f32, f32, f32)> for Rgb {
    fn from((r, g, b): (f32, f32, f32)) -> Self {
        Self::from_f32(r, g, b)
    }
}

use crate::ANSIColorCode;
use crate::TargetGround;
impl ANSIColorCode for Rgb {
    fn ansi_color_code(&self, target: TargetGround) -> String {
        format!("{};2;{};{};{}", target.code() + 8, self.r, self.g, self.b)
    }
}

overload::overload!(
    (lhs: ?Rgb) + (rhs: ?Rgb) -> Rgb {
        Rgb::new(
            lhs.r.saturating_add(rhs.r),
            lhs.g.saturating_add(rhs.g),
            lhs.b.saturating_add(rhs.b)
        )
    }
);

overload::overload!(
    (lhs: ?Rgb) - (rhs: ?Rgb) -> Rgb {
        Rgb::new(
            lhs.r.saturating_sub(rhs.r),
            lhs.g.saturating_sub(rhs.g),
            lhs.b.saturating_sub(rhs.b)
        )
    }
);

overload::overload!(
    (lhs: ?Rgb) * (rhs: ?f32) -> Rgb {
        Rgb::new(
            (lhs.r as f32 * rhs.clamp(0.0, 1.0)) as u8,
            (lhs.g as f32 * rhs.clamp(0.0, 1.0)) as u8,
            (lhs.b as f32 * rhs.clamp(0.0, 1.0)) as u8
        )
    }
);

overload::overload!(
    (lhs: ?f32) * (rhs: ?Rgb) -> Rgb {
        Rgb::new(
            (rhs.r as f32 * lhs.clamp(0.0, 1.0)) as u8,
            (rhs.g as f32 * lhs.clamp(0.0, 1.0)) as u8,
            (rhs.b as f32 * lhs.clamp(0.0, 1.0)) as u8
        )
    }
);

overload::overload!(
    -(rgb: ?Rgb) -> Rgb {
        Rgb::new(
            255 - rgb.r,
            255 - rgb.g,
            255 - rgb.b)
    }
);
