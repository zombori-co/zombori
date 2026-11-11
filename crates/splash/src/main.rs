//! Zombori boot splash: fades in the logo and name, sweeps a glint across the sunglasses,
//! holds briefly, then exits and hands the screen back to the console.

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
mod scene;

#[cfg(target_os = "linux")]
mod display;

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("splash draws through Linux DRM/KMS; build and run it inside the VM");
    std::process::exit(1);
}

#[cfg(target_os = "linux")]
fn main() {
    if let Err(e) = run() {
        eprintln!("splash: {e}");
        std::process::exit(1);
    }
}

#[cfg(target_os = "linux")]
fn run() -> std::io::Result<()> {
    use std::time::{Duration, Instant};
    use theme::{Rgb, VIRTUAL_H, VIRTUAL_W, boot};

    let mut display = display::Display::open(Duration::from_secs(10))?;
    let (w, h) = display.size();

    // Largest whole-number scale that fits, centered.
    let scale = (w / VIRTUAL_W).min(h / VIRTUAL_H).max(1);
    let (ox, oy) = ((w - VIRTUAL_W * scale) / 2, (h - VIRTUAL_H * scale) / 2);

    let scene = scene::Scene::new();
    let mut frame = vec![[0u8; 3]; (VIRTUAL_W * VIRTUAL_H) as usize];
    let mut cleared = [false; 2];
    let mut buffer_index = 0;
    // Page flips normally wait for the monitor's refresh, but some drivers (QEMU's virtio-gpu)
    // complete them immediately. Never draw faster than this.
    let frame_interval = Duration::from_secs(1) / 60;
    let start = Instant::now();
    let mut next_frame = start;

    loop {
        let t = start.elapsed().as_secs_f32();
        scene.render(t, &mut frame);

        let clear = !cleared[buffer_index];
        display.present(|pixels, pitch| {
            if clear {
                fill(pixels, theme::BACKGROUND); // the border outside the scaled scene
            }
            blit(&frame, pixels, pitch, scale, ox, oy);
        })?;
        cleared[buffer_index] = true;
        buffer_index = 1 - buffer_index;

        // After the glint nothing moves any more: keep the last frame up and just wait.
        if t >= boot::GLINT_START + boot::GLINT {
            std::thread::sleep(Duration::from_secs_f32((boot::END - t).max(0.0)));
            return Ok(());
        }

        next_frame += frame_interval;
        let now = Instant::now();
        match next_frame.checked_duration_since(now) {
            Some(wait) => std::thread::sleep(wait),
            None => next_frame = now, // running behind: don't try to catch up
        }
    }

    fn fill(pixels: &mut [u8], [r, g, b]: Rgb) {
        for px in pixels.as_chunks_mut::<4>().0 {
            px.copy_from_slice(&[b, g, r, 0]);
        }
    }

    /// Nearest-neighbor upscale of the virtual screen into the real one.
    fn blit(frame: &[Rgb], pixels: &mut [u8], pitch: usize, scale: u32, ox: u32, oy: u32) {
        let scale = scale as usize;
        for vy in 0..VIRTUAL_H as usize {
            let src = &frame[vy * VIRTUAL_W as usize..][..VIRTUAL_W as usize];
            let first_row = (oy as usize + vy * scale) * pitch;
            let row = &mut pixels[first_row..first_row + pitch];
            for (vx, &[r, g, b]) in src.iter().enumerate() {
                let x0 = (ox as usize + vx * scale) * 4;
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
}
