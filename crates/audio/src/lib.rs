//! Sound for Zombori's boot components: reading WAV files, finding speech in a microphone stream,
//! and (on Linux) playing and recording through ALSA, the kernel's sound API.

pub mod speech;
pub mod wav;

#[cfg(target_os = "linux")]
mod alsa_io;

#[cfg(target_os = "linux")]
pub use alsa_io::{Microphone, play, play_wav};
