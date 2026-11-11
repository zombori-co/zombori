//! Putting pixels on the screen with DRM/KMS: the kernel's own display API, no X11 or Wayland.
//!
//! The plan: find the GPU and the connected monitor, allocate two "dumb" buffers (plain memory
//! we draw into with the CPU), show one while drawing the next, then swap them in sync with the
//! monitor's refresh (a "page flip"). Swapping whole buffers means no tearing.

use std::io;
use std::os::fd::{AsFd, BorrowedFd};
use std::path::Path;
use std::thread::sleep;
use std::time::{Duration, Instant};

use drm::Device as _;
use drm::buffer::DrmFourcc;
use drm::control::{Device as _, Mode, connector, crtc, dumbbuffer::DumbBuffer, framebuffer};
use drm::control::{Event, PageFlipFlags};

struct Card(std::fs::File);

impl AsFd for Card {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}
impl drm::Device for Card {}
impl drm::control::Device for Card {}

struct Buffer {
    db: DumbBuffer,
    fb: framebuffer::Handle,
}

pub struct Display {
    card: Card,
    crtc: crtc::Handle,
    connector: connector::Handle,
    mode: Mode,
    buffers: [Buffer; 2],
    front: usize,
    shown: bool,
    can_flip: bool,
}

/// The generic firmware framebuffer driver. The real GPU driver replaces it during boot, so we
/// wait for the real one and only fall back to this if nothing better shows up.
const FIRMWARE_DRIVER: &str = "simpledrm";

impl Display {
    /// Wait (up to `timeout`) for a GPU with a connected monitor, then take it over.
    pub fn open(timeout: Duration) -> io::Result<Self> {
        let deadline = Instant::now() + timeout;
        loop {
            let past_deadline = Instant::now() >= deadline;
            if let Some(display) = Self::try_open(past_deadline)? {
                return Ok(display);
            }
            if past_deadline {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "no usable display found",
                ));
            }
            sleep(Duration::from_millis(50));
        }
    }

    fn try_open(accept_firmware: bool) -> io::Result<Option<Self>> {
        let Ok(entries) = std::fs::read_dir("/dev/dri") else {
            return Ok(None);
        };
        let mut cards: Vec<_> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("card"))
            })
            .collect();
        cards.sort();
        for path in cards {
            let Ok(card) = open_card(&path) else { continue };
            let driver = card.get_driver()?.name().to_string_lossy().into_owned();
            if driver == FIRMWARE_DRIVER && !accept_firmware {
                continue;
            }
            if let Some(display) = Self::setup(card)? {
                eprintln!("screen: using {} ({driver})", path.display());
                return Ok(Some(display));
            }
        }
        Ok(None)
    }

    fn setup(card: Card) -> io::Result<Option<Self>> {
        let res = card.resource_handles()?;
        let Some(conn) = res
            .connectors()
            .iter()
            .filter_map(|&c| card.get_connector(c, true).ok())
            .find(|c| c.state() == connector::State::Connected && !c.modes().is_empty())
        else {
            return Ok(None);
        };
        let mode = conn.modes()[0]; // the first mode is the monitor's preferred one

        // A CRTC is the scan-out engine that reads a buffer and sends it to the monitor.
        // Use the one already driving this connector, or any compatible one.
        let current = conn
            .current_encoder()
            .and_then(|e| card.get_encoder(e).ok())
            .and_then(|e| e.crtc());
        let crtc = match current {
            Some(c) => c,
            None => {
                let Some(crtc) = conn
                    .encoders()
                    .iter()
                    .filter_map(|&e| card.get_encoder(e).ok())
                    .flat_map(|e| res.filter_crtcs(e.possible_crtcs()))
                    .next()
                else {
                    return Ok(None);
                };
                crtc
            }
        };

        let _ = card.acquire_master_lock(); // usually automatic for the first opener
        let (w, h) = mode.size();
        let new_buffer = || -> io::Result<Buffer> {
            let db = card.create_dumb_buffer((w.into(), h.into()), DrmFourcc::Xrgb8888, 32)?;
            let fb = card.add_framebuffer(&db, 24, 32)?;
            Ok(Buffer { db, fb })
        };
        let buffers = [new_buffer()?, new_buffer()?];

        Ok(Some(Self {
            card,
            crtc,
            connector: conn.handle(),
            mode,
            buffers,
            front: 0,
            shown: false,
            can_flip: true,
        }))
    }

    pub fn size(&self) -> (u32, u32) {
        let (w, h) = self.mode.size();
        (w.into(), h.into())
    }

    /// Draw into the hidden buffer with `draw(pixels, pitch)`, then put it on screen.
    /// `pixels` is XRGB8888 (bytes B, G, R, X per pixel); `pitch` is bytes per row.
    /// Returns once the new frame is visible, which paces the caller to the refresh rate.
    pub fn present(&mut self, draw: impl FnOnce(&mut [u8], usize)) -> io::Result<()> {
        let back = 1 - self.front;
        {
            let buffer = &mut self.buffers[back];
            let pitch = drm::buffer::Buffer::pitch(&buffer.db) as usize;
            let mut map = self.card.map_dumb_buffer(&mut buffer.db)?;
            draw(map.as_mut(), pitch);
        }
        let fb = self.buffers[back].fb;

        if !self.shown {
            self.set_crtc(fb)?;
            self.shown = true;
        } else if self.can_flip {
            match self
                .card
                .page_flip(self.crtc, fb, PageFlipFlags::EVENT, None)
            {
                Ok(()) => self.wait_for_flip()?,
                Err(e) => {
                    eprintln!("screen: page flip unsupported ({e}), falling back to mode set");
                    self.can_flip = false;
                    self.set_crtc(fb)?;
                }
            }
        } else {
            self.set_crtc(fb)?;
            sleep(Duration::from_millis(16));
        }
        self.front = back;
        Ok(())
    }

    fn set_crtc(&self, fb: framebuffer::Handle) -> io::Result<()> {
        self.card.set_crtc(
            self.crtc,
            Some(fb),
            (0, 0),
            &[self.connector],
            Some(self.mode),
        )
    }

    fn wait_for_flip(&self) -> io::Result<()> {
        loop {
            for event in self.card.receive_events()? {
                if let Event::PageFlip(_) = event {
                    return Ok(());
                }
            }
        }
    }
}

impl Drop for Display {
    fn drop(&mut self) {
        for buffer in &self.buffers {
            let _ = self.card.destroy_framebuffer(buffer.fb);
        }
        // When we close the device the kernel gives the screen back to the text console.
    }
}

fn open_card(path: &Path) -> io::Result<Card> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)?;
    Ok(Card(file))
}
