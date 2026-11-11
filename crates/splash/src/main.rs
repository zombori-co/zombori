//! Zombori boot splash: fades in the logo and name, sweeps a glint across the sunglasses,
//! holds briefly, then exits and hands the screen back to the console.

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
mod scene;

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
    use theme::{VIRTUAL_H, VIRTUAL_W, boot};

    let mut screen = screen::Screen::open(Duration::from_secs(10))?;
    let scene = scene::Scene::new();
    let mut frame = vec![[0u8; 3]; (VIRTUAL_W * VIRTUAL_H) as usize];
    // Page flips normally wait for the monitor's refresh, but some drivers (QEMU's virtio-gpu)
    // complete them immediately. Never draw faster than this.
    let frame_interval = Duration::from_secs(1) / 60;
    let start = Instant::now();
    let mut next_frame = start;

    loop {
        let t = start.elapsed().as_secs_f32();
        scene.render(t, &mut frame);
        screen.show(&frame)?;

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
}
