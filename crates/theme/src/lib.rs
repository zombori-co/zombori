//! The shared look of Zombori: colors, logo, pixel font and boot timing.
//! Everything that draws something (splash, greeter, ...) takes its style from here.

pub type Rgb = [u8; 3];

pub const OS_NAME: &str = "ZOMBORI";

pub const BACKGROUND: Rgb = [13, 11, 20];
pub const TEXT: Rgb = [255, 246, 228];
pub const GLINT: Rgb = [255, 246, 228];

/// The boot screen is laid out on a small "virtual screen" and scaled up by a whole number,
/// so every logo pixel stays a crisp square.
pub const VIRTUAL_W: u32 = 160;
pub const VIRTUAL_H: u32 = 100;

pub mod logo {
    use super::Rgb;

    pub const PNG: &[u8] = include_bytes!("../../../assets/logo/logo-32.png");

    /// Outline color: the glint does not paint over it.
    pub const OUTLINE: Rgb = [26, 16, 32];
    /// Rows of the sunglass lenses, and the columns the glint may light up.
    pub const LENS_ROWS: [u32; 3] = [14, 15, 16];
    pub const LENS_COLUMNS: std::ops::Range<u32> = 9..24;
}

/// Boot animation timeline, in seconds (matches `assets/logo/bootscreen.py`).
pub mod boot {
    pub const BLACK: f32 = 0.6;
    pub const LOGO_FADE: f32 = 1.2;
    pub const TEXT_FADE: f32 = 1.0;
    pub const PAUSE: f32 = 0.7;
    pub const GLINT: f32 = 1.0;
    pub const HOLD: f32 = 2.2;

    /// The name starts fading in before the logo has finished.
    pub const TEXT_START: f32 = BLACK + LOGO_FADE * 0.6;
    pub const GLINT_START: f32 = TEXT_START + TEXT_FADE + PAUSE;
    pub const END: f32 = GLINT_START + GLINT + HOLD;
}

/// Smoothstep: eases in and out, so fades have no visible start or stop.
pub fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Blend from the background toward `color`; `k` runs 0.0..=1.0.
pub fn fade(color: Rgb, k: f32) -> Rgb {
    std::array::from_fn(|i| {
        let b = BACKGROUND[i] as f32;
        (b + (color[i] as f32 - b) * k).round() as u8
    })
}

/// An RGBA image.
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<[u8; 4]>,
}

impl Image {
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        self.pixels[(y * self.width + x) as usize]
    }
}

/// Decode the logo. It is embedded at compile time, so this cannot fail at runtime
/// unless the asset itself is broken.
pub fn logo() -> Image {
    decode_png(logo::PNG)
}

/// Decode a PNG embedded at compile time (with `include_bytes!`) into RGBA pixels. Panics only if
/// the asset itself is broken, which its crate's tests should catch.
pub fn decode_png(bytes: &[u8]) -> Image {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::ALPHA);
    let mut reader = decoder.read_info().expect("PNG header");
    let mut buf = vec![0; reader.output_buffer_size().expect("PNG size")];
    let info = reader.next_frame(&mut buf).expect("PNG data");
    let pixels = buf[..info.buffer_size()].as_chunks::<4>().0.to_vec();
    Image {
        width: info.width,
        height: info.height,
        pixels,
    }
}

/// 5x7 pixel font. Only the glyphs we use so far.
pub const GLYPH_W: u32 = 5;
pub const GLYPH_H: u32 = 7;

#[rustfmt::skip]
pub fn glyph(ch: char) -> Option<[&'static str; 7]> {
    Some(match ch {
        'Z' => ["#####", "....#", "...#.", "..#..", ".#...", "#....", "#####"],
        'O' => [".###.", "#...#", "#...#", "#...#", "#...#", "#...#", ".###."],
        'M' => ["#...#", "##.##", "#.#.#", "#.#.#", "#...#", "#...#", "#...#"],
        'B' => ["####.", "#...#", "#...#", "####.", "#...#", "#...#", "####."],
        'R' => ["####.", "#...#", "#...#", "####.", "#.#..", "#..#.", "#...#"],
        'I' => ["#####", "..#..", "..#..", "..#..", "..#..", "..#..", "#####"],
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logo_decodes_to_32x32() {
        let logo = logo();
        assert_eq!((logo.width, logo.height), (32, 32));
        assert_eq!(logo.pixels.len(), 32 * 32);
    }

    #[test]
    fn name_has_glyphs() {
        assert!(OS_NAME.chars().all(|c| glyph(c).is_some()));
    }
}
