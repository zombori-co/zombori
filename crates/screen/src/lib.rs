//! Full-screen pixel graphics for Zombori's boot components. Each component draws a frame on the
//! small virtual screen (`theme::VIRTUAL_W` x `theme::VIRTUAL_H`); this crate scales it up by a
//! whole number and puts it on the monitor through DRM/KMS.

pub mod scale;

#[cfg(target_os = "linux")]
mod display;

#[cfg(target_os = "linux")]
pub use display::Display;

#[cfg(target_os = "linux")]
pub use screen::Screen;

#[cfg(target_os = "linux")]
mod screen {
    use std::io;
    use std::time::Duration;

    use theme::Rgb;

    use crate::display::Display;
    use crate::scale::{Layout, blit, fill};

    /// The monitor, showing whole virtual-screen frames.
    pub struct Screen {
        display: Display,
        layout: Layout,
        cleared: [bool; 2],
        buffer: usize,
    }

    impl Screen {
        /// Wait (up to `timeout`) for a GPU with a connected monitor, then take it over.
        pub fn open(timeout: Duration) -> io::Result<Self> {
            let display = Display::open(timeout)?;
            let (w, h) = display.size();
            Ok(Self {
                display,
                layout: Layout::fit(w, h),
                cleared: [false; 2],
                buffer: 0,
            })
        }

        /// Put a virtual-screen frame on the monitor. Returns once it is visible.
        pub fn show(&mut self, frame: &[Rgb]) -> io::Result<()> {
            let clear = !self.cleared[self.buffer];
            let layout = self.layout;
            self.display.present(|pixels, pitch| {
                if clear {
                    fill(pixels, theme::BACKGROUND); // the border outside the scaled scene
                }
                blit(frame, pixels, pitch, layout);
            })?;
            self.cleared[self.buffer] = true;
            self.buffer = 1 - self.buffer;
            Ok(())
        }
    }
}
