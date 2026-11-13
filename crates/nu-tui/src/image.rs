//! Image previews: decoding, the graphics protocol picked for a run, and the
//! Kitty bookkeeping that lets transmitted images be freed again.
//!
//! `tui preview` and `tui image` hold a decoded [`DynamicImage`] in their
//! state and re-encode it for the terminal whenever their pane or options
//! change. Encoding goes through `ratatui-image`, which draws Kitty, Sixel,
//! and iTerm2 graphics into ratatui's cell buffer and falls back to half
//! blocks elsewhere. Kitty is the only protocol with terminal-side state:
//! an image stays in the terminal's memory under its id until deleted, so
//! ids are handed out here and the automatic ones are deleted when the TUI
//! exits ([`kitty_delete`]).

use crate::widget::Size as PaneSize;
use image::DynamicImage;
use ratatui::layout::Size;
use ratatui_image::picker::{Capability, Picker, ProtocolType};
use ratatui_image::protocol::{
    Protocol, halfblocks::Halfblocks, iterm2::Iterm2, kitty::Kitty, sixel::Sixel,
};
use ratatui_image::{FilterType, FontSize, Resize};
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;

/// How far a preview is zoomed into its image and panned across it, in
/// the pixels of the zoomed image. Zoom 1 is the size `--image-fit` gives.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    pub zoom: f32,
    pub pan: (u32, u32),
    /// How pixels are resampled when the zoom is not 1.
    pub filter: Filter,
}

/// The resampling used to zoom: smooth, or blocky for looking at pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Filter {
    #[default]
    Linear,
    Nearest,
}

impl Filter {
    pub fn name(self) -> &'static str {
        match self {
            Self::Linear => "linear",
            Self::Nearest => "nearest",
        }
    }

    pub fn toggled(self) -> Self {
        match self {
            Self::Linear => Self::Nearest,
            Self::Nearest => Self::Linear,
        }
    }

    fn resampling(self) -> FilterType {
        match self {
            Self::Linear => FilterType::Triangle,
            Self::Nearest => FilterType::Nearest,
        }
    }
}

impl View {
    pub const MIN_ZOOM: f32 = 0.05;
    pub const MAX_ZOOM: f32 = 32.0;
    /// One key press zooms by this factor.
    pub const ZOOM_STEP: f32 = 1.25;

    pub fn zoom_by(&mut self, factor: f32) {
        self.zoom = (self.zoom * factor).clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
    }

    /// Back to the first view, keeping the choice of filter.
    pub fn reset(&mut self) {
        *self = Self {
            filter: self.filter,
            ..Self::default()
        };
    }

    /// Move the window by `(dx, dy)` zoomed pixels; the next encode clamps
    /// it to the image.
    pub fn pan_by(&mut self, dx: i64, dy: i64) {
        let shift = |px: u32, by: i64| (i64::from(px) + by).max(0) as u32;
        self.pan = (shift(self.pan.0, dx), shift(self.pan.1, dy));
    }
}

impl Default for View {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: (0, 0),
            filter: Filter::default(),
        }
    }
}

/// Largest image file decoded for a preview.
pub const MAX_IMAGE_BYTES: u64 = 32 * 1024 * 1024;

/// How an image is sized inside its pane (`--image-fit`). Every mode keeps
/// the aspect ratio.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageFit {
    /// Shrink to the box, never grow.
    #[default]
    Fit,
    /// Fill the box, growing a small image.
    Scale,
    /// One pixel per pixel; what does not fit is clipped at the right and
    /// bottom.
    Crop,
}

impl ImageFit {
    pub const NAMES: [&str; 3] = ["fit", "scale", "crop"];

    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "fit" => Some(Self::Fit),
            "scale" => Some(Self::Scale),
            "crop" => Some(Self::Crop),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Fit => "fit",
            Self::Scale => "scale",
            Self::Crop => "crop",
        }
    }
}

/// `--image-fit`, `--image-width`, and `--image-height`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImageOptions {
    pub fit: ImageFit,
    /// Caps on the box the image is sized into: cells, or a percentage of
    /// the pane. Only `Size::Length` and `Size::Percent` are accepted.
    pub max_width: Option<PaneSize>,
    pub max_height: Option<PaneSize>,
}

impl ImageOptions {
    /// The box an image is sized into: the pane, capped by the options.
    pub fn target(&self, pane: (u16, u16)) -> Size {
        let cap = |limit: Option<PaneSize>, pane: u16| match limit {
            Some(PaneSize::Length(n)) => n.min(pane),
            Some(PaneSize::Percent(p)) => (u32::from(pane) * u32::from(p) / 100) as u16,
            _ => pane,
        };
        Size::new(cap(self.max_width, pane.0), cap(self.max_height, pane.1))
    }
}

/// The graphics protocol for this run, plus the Kitty ids handed out.
#[derive(Debug, Clone)]
pub struct Images {
    picker: Picker,
    next_id: u32,
}

impl Images {
    pub fn new(picker: Picker) -> Self {
        Self {
            picker,
            next_id: random_id_base(),
        }
    }

    /// Half blocks, for `tui debug` and for terminals that answer no
    /// graphics query.
    pub fn headless() -> Self {
        Self::new(Picker::halfblocks())
    }

    /// Force a protocol the terminal was not asked about or did not report.
    pub fn with_protocol(mut self, protocol: ProtocolType) -> Self {
        self.picker.set_protocol_type(protocol);
        self
    }

    pub fn protocol_name(&self) -> &'static str {
        match self.picker.protocol_type() {
            ProtocolType::Halfblocks => "halfblocks",
            ProtocolType::Sixel => "sixel",
            ProtocolType::Kitty => "kitty",
            ProtocolType::Iterm2 => "iterm2",
        }
    }

    pub fn is_kitty(&self) -> bool {
        self.picker.protocol_type() == ProtocolType::Kitty
    }

    /// Whether Kitty transmissions go through POSIX shared memory rather
    /// than base64 over the terminal link.
    pub fn uses_shared_memory(&self) -> bool {
        self.is_kitty()
            && self
                .picker
                .capabilities()
                .contains(&Capability::KittySharedMemory)
    }

    pub fn is_tmux(&self) -> bool {
        self.picker.tmux_detected()
    }

    /// The next automatic Kitty id.
    pub fn alloc_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id = next_kitty_id(id);
        id
    }

    /// The terminal's cell size in pixels, as the picker found it.
    pub fn font_size(&self) -> FontSize {
        self.picker.font_size()
    }

    /// Encode `image` for the terminal, sized by `options` inside `pane`
    /// and then zoomed and panned by `view`, whose pan is clamped to the
    /// image. Under Kitty it is transmitted as `kitty_id` on its first
    /// render.
    pub fn encode(
        &self,
        image: &DynamicImage,
        options: &ImageOptions,
        pane: (u16, u16),
        view: &mut View,
        kitty_id: u32,
    ) -> Result<Protocol, String> {
        let target = options.target(pane);
        if target.width == 0 || target.height == 0 {
            return Err("no room for the image".into());
        }
        let font = self.picker.font_size();
        let (box_w, box_h) = (
            f64::from(target.width) * f64::from(font.width),
            f64::from(target.height) * f64::from(font.height),
        );
        let (width, height) = (f64::from(image.width()), f64::from(image.height()));
        // The scale the fit mode gives at zoom 1: `fit` only ever shrinks,
        // `scale` fills the box either way, `crop` is one pixel per pixel.
        let base = match options.fit {
            ImageFit::Fit => (box_w / width).min(box_h / height).min(1.0),
            ImageFit::Scale => (box_w / width).min(box_h / height),
            ImageFit::Crop => 1.0,
        };
        view.zoom = view.zoom.clamp(View::MIN_ZOOM, View::MAX_ZOOM);
        let scale = base * f64::from(view.zoom);
        let zoomed = |px: f64| (px * scale).round().max(1.0) as u32;
        let (zoomed_w, zoomed_h) = (zoomed(width), zoomed(height));
        // The window onto the zoomed image: the box, or the whole image when
        // that is smaller. The pan keeps the window inside the image.
        let window_w = zoomed_w.min(box_w as u32);
        let window_h = zoomed_h.min(box_h as u32);
        view.pan = (
            view.pan.0.min(zoomed_w - window_w),
            view.pan.1.min(zoomed_h - window_h),
        );
        // Cut the window's source region out first, so zooming into a large
        // photo resizes only what is shown.
        let unzoomed = |px: u32| (f64::from(px) / scale).round() as u32;
        let region = image.crop_imm(
            unzoomed(view.pan.0).min(image.width().saturating_sub(1)),
            unzoomed(view.pan.1).min(image.height().saturating_sub(1)),
            unzoomed(window_w).max(1),
            unzoomed(window_h).max(1),
        );
        let shown = if region.width() == window_w && region.height() == window_h {
            region
        } else {
            region.resize_exact(window_w, window_h, view.filter.resampling())
        };
        let size = Size::new(
            window_w.div_ceil(u32::from(font.width)) as u16,
            window_h.div_ceil(u32::from(font.height)) as u16,
        );
        // Pads the window out to whole cells; it already fits, so nothing
        // is resized again.
        let image = Resize::Fit(None).resize(&shown, font, size, None);
        let tmux = self.is_tmux();
        let protocol = match self.picker.protocol_type() {
            ProtocolType::Halfblocks => Halfblocks::new(image, size).map(Protocol::Halfblocks),
            ProtocolType::Sixel => Sixel::new(image, size, tmux).map(Protocol::Sixel),
            ProtocolType::Kitty => {
                // Shared memory when the terminal read the probe's object:
                // one command, no chunk stream, so the old image is replaced
                // without a blank frame. Compression only matters on the
                // base64 fallback, such as over SSH.
                let caps = self.picker.capabilities();
                let compress = caps.contains(&Capability::KittyCompression);
                let shm = caps.contains(&Capability::KittySharedMemory);
                Kitty::new(image, size, kitty_id, tmux, compress, shm).map(Protocol::Kitty)
            }
            ProtocolType::Iterm2 => Iterm2::new(image, size, tmux).map(Protocol::ITerm2),
        };
        protocol.map_err(|err| err.to_string())
    }
}

/// Decode an image. `None` when the bytes are not in a recognised format,
/// so the caller shows them as text or a binary note instead.
pub fn decode(bytes: &[u8]) -> Option<Result<(DynamicImage, String), String>> {
    let format = image::guess_format(bytes).ok()?;
    let name = format!("{format:?}").to_ascii_lowercase();
    Some(
        image::load_from_memory_with_format(bytes, format)
            .map(|image| (image, name))
            .map_err(|err| err.to_string()),
    )
}

/// The sequence that frees Kitty images `ids`, placements and data, from
/// the terminal's memory. Wrapped for tmux passthrough when `tmux`.
/// Free what a Kitty transmission that never reached the terminal still
/// holds. With shared memory (`t=s`) the pixels sit in a POSIX shm object
/// that the terminal unlinks once it has read them; a protocol that is
/// dropped before it is drawn (a zoom step overtaken by the next, a job
/// cancelled after encoding) would leave that object behind until reboot,
/// so its name is read back out of the transmission and unlinked here.
/// Base64 and non-Kitty transmissions hold nothing outside the process,
/// and a protocol that was drawn emits its transmission only once, so
/// drawing it again here yields nothing for one the terminal already has.
/// Returns the name of the object it unlinked.
pub fn release_untransmitted(protocol: &Protocol) -> Option<String> {
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::widgets::Widget;
    let Protocol::Kitty(_) = protocol else {
        return None;
    };
    // The transmission is prefixed to the first placeholder cell drawn.
    let mut buf = Buffer::empty(Rect::new(0, 0, 1, 1));
    ratatui_image::Image::new(protocol).render(buf.area, &mut buf);
    let name = shm_name(buf[(0, 0)].symbol())?;
    #[cfg(unix)]
    {
        nix::sys::mman::shm_unlink(name.as_str()).ok()?;
    }
    Some(name)
}

/// The shared memory object a Kitty transmission names, if it uses one:
/// the base64 payload after `t=s`'s command, up to the ESC that ends it.
fn shm_name(transmission: &str) -> Option<String> {
    use base64::Engine as _;
    let (_, rest) = transmission.split_once("t=s,")?;
    let (_, payload) = rest.split_once(';')?;
    let payload = payload.split('\x1b').next()?;
    let name = base64::engine::general_purpose::STANDARD
        .decode(payload.as_bytes())
        .ok()?;
    String::from_utf8(name).ok()
}

pub fn kitty_delete(ids: &[u32], tmux: bool) -> String {
    let mut out = String::new();
    for id in ids {
        kitty_command(&mut out, &format!("a=d,d=I,i={id},q=2"), tmux);
    }
    out
}

/// Append one Kitty graphics command. Inside tmux it goes in a passthrough
/// wrapper with its escapes doubled, as tmux requires.
fn kitty_command(out: &mut String, command: &str, tmux: bool) {
    let seq = format!("\x1b_G{command}\x1b\\");
    if tmux {
        let _ = write!(out, "\x1bPtmux;{}\x1b\\", seq.replace('\x1b', "\x1b\x1b"));
    } else {
        out.push_str(&seq);
    }
}

/// Kitty ids are nonzero and, kept under 2^24, fit the placeholder's
/// foreground color alone. The base is drawn from the clock and the process
/// id so two shells sharing a terminal are unlikely to collide.
fn random_id_base() -> u32 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    next_kitty_id(nanos ^ std::process::id().rotate_left(13))
}

fn next_kitty_id(after: u32) -> u32 {
    let id = after.wrapping_add(1) & 0x00FF_FFFF;
    if id == 0 { 1 } else { id }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn target_caps_the_pane() {
        let options = ImageOptions {
            max_width: Some(PaneSize::Length(10)),
            max_height: Some(PaneSize::Percent(50)),
            ..ImageOptions::default()
        };
        assert_eq!(options.target((40, 20)), Size::new(10, 10));
        assert_eq!(ImageOptions::default().target((40, 20)), Size::new(40, 20));
        // A cap wider than the pane is the pane.
        let wide = ImageOptions {
            max_width: Some(PaneSize::Length(100)),
            ..ImageOptions::default()
        };
        assert_eq!(wide.target((40, 20)), Size::new(40, 20));
    }

    #[test]
    fn kitty_ids_are_nonzero_and_24_bit() {
        assert_eq!(next_kitty_id(0x00FF_FFFF), 1);
        assert_eq!(next_kitty_id(u32::MAX), 1);
        assert_eq!(next_kitty_id(41), 42);
        let base = random_id_base();
        assert!(base != 0 && base <= 0x00FF_FFFF);
    }

    #[test]
    fn delete_sequence_frees_data_and_wraps_for_tmux() {
        assert_eq!(kitty_delete(&[7], false), "\x1b_Ga=d,d=I,i=7,q=2\x1b\\");
        assert_eq!(
            kitty_delete(&[7], true),
            "\x1bPtmux;\x1b\x1b_Ga=d,d=I,i=7,q=2\x1b\x1b\\\x1b\\"
        );
        assert_eq!(kitty_delete(&[], false), "");
    }

    #[test]
    fn decode_recognises_png_and_rejects_text() {
        let png = image_fixture();
        let (image, format) = decode(&png).expect("png").expect("decodes");
        assert_eq!(
            (image.width(), image.height(), format.as_str()),
            (8, 4, "png")
        );
        assert!(decode(b"hello world").is_none());
    }

    #[test]
    fn encode_fits_scales_and_crops() {
        let images = Images::headless();
        let (image, _) = decode(&image_fixture()).expect("png").expect("decodes");
        // Half blocks: 10x20 font, so an 8x4 image is one cell wide and one
        // high (rounded up), and `fit` never grows it.
        let fit = images
            .encode(
                &image,
                &ImageOptions::default(),
                (40, 10),
                &mut View::default(),
                1,
            )
            .expect("fit");
        assert_eq!(fit.size(), Size::new(1, 1));
        // `scale` grows it to the box, keeping 2:1: a 40x20 box is 400x400
        // pixels, so the image becomes 400x200, which is 40 by 10 cells.
        let scale = images
            .encode(
                &image,
                &ImageOptions {
                    fit: ImageFit::Scale,
                    ..ImageOptions::default()
                },
                (40, 20),
                &mut View::default(),
                1,
            )
            .expect("scale");
        assert_eq!(scale.size(), Size::new(40, 10));
        let crop = images
            .encode(
                &image,
                &ImageOptions {
                    fit: ImageFit::Crop,
                    ..ImageOptions::default()
                },
                (40, 10),
                &mut View::default(),
                1,
            )
            .expect("crop");
        assert_eq!(crop.size(), Size::new(1, 1));
        let err = images
            .encode(
                &image,
                &ImageOptions::default(),
                (0, 10),
                &mut View::default(),
                1,
            )
            .err()
            .expect("no room");
        assert!(err.contains("no room"));
    }

    #[test]
    fn zoom_grows_the_window_and_pan_is_clamped() {
        let images = Images::headless();
        let (image, _) = decode(&image_fixture()).expect("png").expect("decodes");
        // Zoomed 10x, the 8x4 image is 80x40 pixels: 8 columns by 2 rows.
        let mut view = View {
            zoom: 10.0,
            ..View::default()
        };
        let zoomed = images
            .encode(&image, &ImageOptions::default(), (40, 10), &mut view, 1)
            .expect("zoomed");
        assert_eq!(zoomed.size(), Size::new(8, 2));
        // A 3-column box shows a 30-pixel window; the pan cannot pass the
        // image's right edge at 80 - 30.
        let mut view = View {
            zoom: 10.0,
            pan: (1000, 1000),
            ..View::default()
        };
        let panned = images
            .encode(&image, &ImageOptions::default(), (3, 10), &mut view, 1)
            .expect("panned");
        assert_eq!(panned.size(), Size::new(3, 2));
        assert_eq!(view.pan, (50, 0));
        // Zoom is clamped to its range.
        let mut view = View {
            zoom: 1000.0,
            ..View::default()
        };
        images
            .encode(&image, &ImageOptions::default(), (3, 10), &mut view, 1)
            .expect("clamped");
        assert_eq!(view.zoom, View::MAX_ZOOM);
        let mut view = View::default();
        view.zoom_by(View::ZOOM_STEP);
        view.pan_by(-5, 7);
        assert_eq!((view.zoom, view.pan), (1.25, (0, 7)));
    }

    #[test]
    fn shm_name_is_read_out_of_the_transmission() {
        let seq = "\x1b_Gq=2,i=7,a=T,U=1,f=32,t=s,s=8,v=4,c=1,r=1,m=0;L3J0dWktYWJj\x1b\\\u{10EEEE}";
        assert_eq!(shm_name(seq).as_deref(), Some("/rtui-abc"));
        let direct = "\x1b_Gq=2,i=7,a=T,U=1,f=32,t=d,s=8,v=4,m=0;AAAA\x1b\\\u{10EEEE}";
        assert_eq!(shm_name(direct), None);
        assert_eq!(shm_name("\u{10EEEE}"), None);
    }

    #[cfg(unix)]
    #[test]
    fn an_undrawn_shared_memory_transmission_is_unlinked() {
        use nix::fcntl::OFlag;
        use nix::sys::mman::shm_open;
        use nix::sys::stat::Mode;
        let (image, _) = decode(&image_fixture()).expect("png").expect("decodes");
        let protocol = Kitty::new(image, Size::new(1, 1), 7, false, false, true)
            .map(Protocol::Kitty)
            .expect("kitty");
        let name = release_untransmitted(&protocol).expect("an object was unlinked");
        assert!(name.starts_with("/rtui-"), "{name}");
        assert!(
            shm_open(name.as_str(), OFlag::O_RDONLY, Mode::empty()).is_err(),
            "{name} still exists"
        );
        // Drawn protocols emit their transmission once, so nothing is left
        // to release; a half-block protocol never had anything.
        assert_eq!(release_untransmitted(&protocol), None);
        let (image, _) = decode(&image_fixture()).expect("png").expect("decodes");
        let blocks = Halfblocks::new(image, Size::new(1, 1))
            .map(Protocol::Halfblocks)
            .expect("halfblocks");
        assert_eq!(release_untransmitted(&blocks), None);
    }

    /// An 8x4 PNG, red on the left and blue on the right.
    pub(crate) fn image_fixture() -> Vec<u8> {
        const HEX: &str = "89504e470d0a1a0a0000000d4948445200000008000000040806000000b3cd7ef00000004a494441547801edc003a0245996c6f1ff77ee8dc8cca7724b63ae6ddbb66ddbb66ddbb66d698c9e964aaf9e323322eef976b76a7aa6873b6bd5af1acc0308f300042f1cc10b47f0c2f18f772e0309d0272ed70000000049454e44ae426082";
        (0..HEX.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&HEX[i..i + 2], 16).expect("hex"))
            .collect()
    }
}
