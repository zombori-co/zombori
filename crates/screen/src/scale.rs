//! Scaling the virtual screen up to the real one. Plain memory work, so it runs (and is tested)
//! anywhere.

use theme::{Rgb, VIRTUAL_H, VIRTUAL_W};

/// Where the scaled virtual screen sits on the real one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layout {
    pub scale: u32,
    pub ox: u32,
    pub oy: u32,
}

impl Layout {
    /// The largest whole-number scale that fits a `w` x `h` screen, centered.
    pub fn fit(w: u32, h: u32) -> Self {
        let scale = (w / VIRTUAL_W).min(h / VIRTUAL_H).max(1);
        Self {
            scale,
            ox: w.saturating_sub(VIRTUAL_W * scale) / 2,
            oy: h.saturating_sub(VIRTUAL_H * scale) / 2,
        }
    }
}

/// Fill an XRGB8888 buffer (bytes B, G, R, X per pixel) with one color.
pub fn fill(pixels: &mut [u8], [r, g, b]: Rgb) {
    for px in pixels.as_chunks_mut::<4>().0 {
        px.copy_from_slice(&[b, g, r, 0]);
    }
}

/// Nearest-neighbor upscale of the virtual screen into an XRGB8888 buffer with `pitch` bytes per
/// row.
pub fn blit(frame: &[Rgb], pixels: &mut [u8], pitch: usize, layout: Layout) {
    let scale = layout.scale as usize;
    for vy in 0..VIRTUAL_H as usize {
        let src = &frame[vy * VIRTUAL_W as usize..][..VIRTUAL_W as usize];
        let first_row = (layout.oy as usize + vy * scale) * pitch;
        let row = &mut pixels[first_row..first_row + pitch];
        for (vx, &[r, g, b]) in src.iter().enumerate() {
            let x0 = (layout.ox as usize + vx * scale) * 4;
            for px in row[x0..x0 + scale * 4].as_chunks_mut::<4>().0 {
                px.copy_from_slice(&[b, g, r, 0]);
            }
        }
        // The other rows of this scaled pixel row are copies of the first.
        let (done, rest) = pixels.split_at_mut(first_row + pitch);
        let first = &done[first_row..];
        for copy in rest.chunks_exact_mut(pitch).take(scale - 1) {
            copy.copy_from_slice(first);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fits_common_screens() {
        assert_eq!(
            Layout::fit(1280, 800),
            Layout {
                scale: 8,
                ox: 0,
                oy: 0
            }
        );
        // 1920 / 160 = 12, 1080 / 100 = 10: limited by the height, centered horizontally.
        assert_eq!(
            Layout::fit(1920, 1080),
            Layout {
                scale: 10,
                ox: 160,
                oy: 40
            }
        );
    }

    #[test]
    fn blit_scales_each_pixel_into_a_square() {
        let layout = Layout::fit(VIRTUAL_W * 2, VIRTUAL_H * 2);
        let pitch = (VIRTUAL_W * 2 * 4) as usize;
        let mut frame = vec![[0, 0, 0]; (VIRTUAL_W * VIRTUAL_H) as usize];
        frame[(VIRTUAL_W + 1) as usize] = [10, 20, 30]; // virtual pixel (1, 1)
        let mut pixels = vec![0xff; pitch * (VIRTUAL_H * 2) as usize];
        blit(&frame, &mut pixels, pitch, layout);
        for (x, y) in [(2, 2), (3, 2), (2, 3), (3, 3)] {
            let i = y * pitch + x * 4;
            assert_eq!(&pixels[i..i + 4], &[30, 20, 10, 0], "({x}, {y})");
        }
        assert_eq!(&pixels[4 * pitch + 8..4 * pitch + 12], &[0, 0, 0, 0]);
    }
}
