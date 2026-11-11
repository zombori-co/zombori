//! What the boot screen looks like at a given moment, drawn on the small virtual screen.
//! No hardware involved, so this runs (and is tested) anywhere.

use theme::{Image, Rgb, VIRTUAL_H, VIRTUAL_W, boot, ease, fade};

pub struct Scene {
    logo: Image,
}

/// Logo and name are centered together: logo, an 8 px gap, then the text.
const GAP: u32 = 8;

impl Scene {
    pub fn new() -> Self {
        Self {
            logo: theme::logo(),
        }
    }

    /// Render the frame at `t` seconds since the splash started.
    pub fn render(&self, t: f32, out: &mut [Rgb]) {
        out.fill(theme::BACKGROUND);

        let logo_k = ease((t - boot::BLACK) / boot::LOGO_FADE);
        let text_k = ease((t - boot::TEXT_START) / boot::TEXT_FADE);
        let g = (t - boot::GLINT_START) / boot::GLINT;
        let glint_x = (0.0..1.0)
            .contains(&g)
            .then(|| 6 + (g * 24.0).round() as i32);

        let block_h = self.logo.height + GAP + theme::GLYPH_H;
        let logo_y = (VIRTUAL_H - block_h) / 2;
        let text_y = logo_y + self.logo.height + GAP;

        self.draw_logo(
            out,
            (VIRTUAL_W - self.logo.width) / 2,
            logo_y,
            logo_k,
            glint_x,
        );
        draw_text(out, theme::OS_NAME, text_y, text_k);
    }

    fn draw_logo(&self, out: &mut [Rgb], x0: u32, y0: u32, k: f32, glint_x: Option<i32>) {
        for y in 0..self.logo.height {
            for x in 0..self.logo.width {
                let [r, g, b, a] = self.logo.pixel(x, y);
                if a == 0 {
                    continue;
                }
                let mut color = [r, g, b];
                if let Some(gx) = glint_x
                    && in_glint(x, y, gx)
                    && color != theme::logo::OUTLINE
                {
                    color = theme::GLINT;
                }
                out[((y0 + y) * VIRTUAL_W + x0 + x) as usize] = fade(color, k);
            }
        }
    }
}

/// The glint is a 2 px wide diagonal band across the lens rows, leaning right toward the top.
fn in_glint(x: u32, y: u32, glint_x: i32) -> bool {
    let Some(row) = theme::logo::LENS_ROWS.iter().position(|&r| r == y) else {
        return false;
    };
    let left = glint_x - row as i32;
    theme::logo::LENS_COLUMNS.contains(&x) && (left..left + 2).contains(&(x as i32))
}

fn draw_text(out: &mut [Rgb], text: &str, y0: u32, k: f32) {
    let advance = theme::GLYPH_W + 1;
    let width = text.len() as u32 * advance - 1;
    let x0 = (VIRTUAL_W - width) / 2;
    let color = fade(theme::TEXT, k);
    for (i, ch) in text.chars().enumerate() {
        let Some(rows) = theme::glyph(ch) else {
            continue;
        };
        for (gy, row) in rows.iter().enumerate() {
            for (gx, px) in row.bytes().enumerate() {
                if px == b'#' {
                    let x = x0 + i as u32 * advance + gx as u32;
                    out[((y0 + gy as u32) * VIRTUAL_W + x) as usize] = color;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame_at(t: f32) -> Vec<Rgb> {
        let mut out = vec![[0; 3]; (VIRTUAL_W * VIRTUAL_H) as usize];
        Scene::new().render(t, &mut out);
        out
    }

    #[test]
    fn starts_black() {
        assert!(frame_at(0.0).iter().all(|&p| p == theme::BACKGROUND));
    }

    /// The final frame must match the approved mockup pixel for pixel.
    #[test]
    fn final_frame_matches_mockup() {
        let png = include_bytes!("../../../assets/logo/bootscreen.png");
        let mut reader = png::Decoder::new(std::io::Cursor::new(&png[..]))
            .read_info()
            .unwrap();
        let mut buf = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut buf).unwrap();
        let scale = info.width / VIRTUAL_W;
        let frame = frame_at(boot::END);
        for y in 0..VIRTUAL_H {
            for x in 0..VIRTUAL_W {
                let i = (((y * scale) * info.width + x * scale) * 3) as usize;
                let expected = [buf[i], buf[i + 1], buf[i + 2]];
                assert_eq!(
                    frame[(y * VIRTUAL_W + x) as usize],
                    expected,
                    "pixel ({x}, {y})"
                );
            }
        }
    }
}
