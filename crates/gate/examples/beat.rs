//! Show what the gate makes of a music clip: its beat, where the music starts, and the tempo found
//! in each stretch of the clip, to see where steady music starts:
//!   cargo run -p gate --release --example beat -- assets/sounds/local/celebrate.wav

use gate::beat::{detect, music_start};

fn main() {
    let path = std::env::args().nth(1).expect("usage: beat <wav>");
    let wav = audio::wav::parse(&std::fs::read(&path).unwrap()).unwrap();
    let mono = wav.mono_f32();
    let rate = wav.rate as usize;
    println!(
        "{path}: {:.1} s, {} Hz, {} ch",
        wav.seconds(),
        wav.rate,
        wav.channels
    );

    let whole = detect(&mono, wav.rate);
    println!(
        "whole clip: {:.1} BPM, first beat at {:.2} s; the head starts swaying at {:.1} s",
        whole.bpm,
        whole.first,
        music_start(&mono, wav.rate)
    );

    println!("tempo in 6 s windows:");
    let window = 6 * rate;
    let mut start = 0;
    while start + window <= mono.len() {
        let beat = detect(&mono[start..start + window], wav.rate);
        let at = start as f32 / rate as f32;
        println!("  {at:5.1}–{:5.1} s  {:6.1} BPM", at + 6.0, beat.bpm);
        start += 2 * rate;
    }
}
