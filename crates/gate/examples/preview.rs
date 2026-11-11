//! Render the gate's views to PNG files, to look at the animation without a VM:
//!   cargo run -p gate --example preview -- <out-dir> [music.wav]
//! Writes one 4x-scaled frame every 1/25 s for each stage. With a music clip, the celebration is
//! timed to it exactly as the gate would (beat and music start found the same way) and lasts as
//! long as the gate would play it; frames for it alone go to <out-dir>/celebration/.

use gate::beat::{self, Beat};
use gate::scene::{Scene, View};
use theme::{VIRTUAL_H, VIRTUAL_W};

const SCALE: u32 = 4;
const FPS: f32 = 25.0;
/// The gate plays at most this much of the clip (MAX_CELEBRATION_S in main.rs).
const MAX_CELEBRATION_S: f32 = 25.0;

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args.next().expect("usage: preview <out-dir> [music.wav]");
    let (beat, music, seconds) = match args.next() {
        Some(path) => {
            let wav = audio::wav::parse(&std::fs::read(&path).unwrap()).unwrap();
            let mono = wav.mono_f32();
            let beat = beat::detect(&mono, wav.rate);
            let music = beat::music_start(&mono, wav.rate);
            println!(
                "{path}: music from {music:.1} s, {:.1} BPM, first beat at {:.2} s",
                beat.bpm, beat.first
            );
            (beat, music, wav.seconds().min(MAX_CELEBRATION_S))
        }
        None => (
            Beat {
                bpm: 126.0,
                first: 0.3,
            },
            0.0,
            8.0,
        ),
    };

    let scene = Scene::new();
    let mut frame = vec![[0u8; 3]; (VIRTUAL_W * VIRTUAL_H) as usize];
    let mut render = |subdir: &str, name: &str, length: f32, view: &dyn Fn(f32) -> View| {
        let out = format!("{dir}/{subdir}");
        std::fs::create_dir_all(&out).unwrap();
        let mut drawing = std::time::Duration::ZERO;
        let frames = (length * FPS) as usize;
        for i in 0..frames {
            let start = std::time::Instant::now();
            scene.render(view(i as f32 / FPS), &mut frame);
            drawing += start.elapsed();
            write(&format!("{out}/{name}-{i:04}.png"), &frame);
        }
        let ms = drawing.as_secs_f32() * 1000.0 / frames.max(1) as f32;
        println!("{name}: {frames} frames, {ms:.1} ms each to draw");
    };
    render("", "1-asking", 1.0, &|t| View::Asking { t, first: true });
    render("", "2-listening", 2.0, &|t| View::Listening {
        t,
        level: (t * 2.3).sin().abs(),
    });
    render("", "3-thinking", 1.0, &|t| View::Thinking { t });
    render("", "4-wrong", 1.2, &|t| View::Wrong { t });
    render("celebration", "frame", seconds, &|t| View::Celebrating {
        t,
        beat,
        music,
    });
}

fn write(path: &str, frame: &[[u8; 3]]) {
    let (w, h) = (VIRTUAL_W * SCALE, VIRTUAL_H * SCALE);
    let mut data = Vec::with_capacity((w * h * 3) as usize);
    for y in 0..h {
        for x in 0..w {
            data.extend_from_slice(&frame[((y / SCALE) * VIRTUAL_W + x / SCALE) as usize]);
        }
    }
    let file = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
    let mut encoder = png::Encoder::new(file, w, h);
    encoder.set_color(png::ColorType::Rgb);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&data)
        .unwrap();
}
